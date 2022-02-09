## Mailbox Processor Readme

Mailbox Processor is a small little async actor abstraction inspired by the FSharp Mailbox Processor which in turn was inspired by erlang.

Why use this abstraction instead of something more comprehensive like actix.  Mainly because it's simple and small, and sometimes you just need a simple abstraction.

This abstraction sees a lot of usage in the fsharp community and has proven to be very useful there.

So what is this thing?  It's probably the most basic representation of an actor you could come up with.  It's basically a queue sitting in front of an event loop.  It's async nature means that when it's not actively processing a message it takes up very few resources so you can spin up a bunch of them if you want (not sure how many but hundreds of thousands wouldn't surprise me).  You provide the function that processes the messages coming in from the queue.  The state is passed in as a parameter to the function, and the return value of the function is the updated state.  You can send responses back to the original caller in a channel or just send messages in a "fire_and_forget" fashion.

For the most part I've been using this abstraction to synchronize things.  I used it once to accept a bunch of requests coming from endpoints and serialize the processing of the output.  I used it another time to coordinate the taking of snapshots of data inbetween processing runs that manipulate the data coming in the form of requests to the system.

It's pretty useful for solving all sorts of concurrency problems...

This mailbox processor does have on thing the fsharp original doesn't have... It's based on a bounded channel(queue), which means you can to some extent have some control over backpressure.

I based this on async-std, but it would be easy to port to tokio (if that even makes sense).  If someone runs into a problem using it along side tokio, I would accept a pull request for a tokio version or I would be willing to build a tokio version (shouldn't take much work).

### Here is an example of a counter using the mailbox processor

```rust

        enum SendMessageTypes {
            Increment(i32),
            GetCurrentCount,
            Decrement(i32),
        }

        let mb = MailboxProcessor::<SendMessageTypes, i32>::new( 
            BufferSize::Default, 
            0,  
            |msg, state, reply_channel| async move {
                match msg {
                    SendMessageTypes::Increment(x) => {
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(state + x).await.unwrap()
                        })).await;
                        state + x
                    },
                    SendMessageTypes::GetCurrentCount => {
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(state).await.unwrap()
                        })).await;
                        state
                    },
                    SendMessageTypes::Decrement(x) => {
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(state - x).await.unwrap()
                        })).await;
                        state - x
                    },
                }
            }
        ).await;

        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 0);

        mb.fire_and_forget(SendMessageTypes::Increment(55)).await.unwrap();
        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 55);

        mb.fire_and_forget(SendMessageTypes::Increment(55)).await.unwrap();
        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 110);

        mb.fire_and_forget(SendMessageTypes::Decrement(10)).await.unwrap();
        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 100);

        assert_eq!(mb.send(SendMessageTypes::Increment(55)).await.unwrap(), 155);
        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 155);

```
