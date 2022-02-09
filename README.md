Mailbox Processor is a small little actor abstraction inspired by the FSharp Mailbox Processor which in turn was inspired by erlang.

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
                        state + x
                    },
                    SendMessageTypes::GetCurrentCount => {
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(state).await.unwrap()
                        })).await;
                        state
                    },
                    SendMessageTypes::Decrement(x) => {
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

        assert_eq!(mb.send(SendMessageTypes::Increment(55)).await, Err("the response channel is closed (did you mean to call fire_and_forget() rather than send())"));
        assert_eq!(mb.send(SendMessageTypes::GetCurrentCount).await.unwrap(), 155);
```
