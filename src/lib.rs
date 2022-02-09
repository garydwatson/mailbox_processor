use async_std::prelude::*;
use async_std::channel::*;
use async_std::task;

pub struct MailboxProcessor<Msg, ReplyMsg> {
    message_sender: Sender<(Msg, Option<Sender<ReplyMsg>>)>,
}

pub enum BufferSize {
    Default,
    Size(usize),
}

impl BufferSize {
    fn unwrap_or(&self, default_value: usize) -> usize {
        match self {
            BufferSize::Default => default_value,
            BufferSize::Size(x) => *x,
        }

    }
}

impl<Msg: std::marker::Send + 'static, ReplyMsg: std::marker::Send + 'static> MailboxProcessor<Msg, ReplyMsg> {
    pub async fn new<State: std::marker::Send + 'static, F: Future<Output = State> + std::marker::Send>
    (
        buffer_size: BufferSize,
        initial_state: State, 
        message_processing_function: impl Fn(Msg, State, Option<Sender<ReplyMsg>>) -> F + std::marker::Send + 'static + std::marker::Sync,
    ) 
    -> Self {
        let (s,r) = bounded(buffer_size.unwrap_or(1_000));
        task::spawn(async move { 
            let mut state = initial_state;
            loop {
                match r.recv().await {
                    Err(_) => break,  //the channel was closed so bail
                    Ok((msg, reply_channel)) => {
                        state = message_processing_function(msg, state, reply_channel).await;
                    },
                }
            }
        });
        MailboxProcessor {
            message_sender: s,
        }
    }
    pub async fn send(&self, msg:Msg) -> Result<ReplyMsg, &str> {
        let (s, r) = bounded(1);
        match self.message_sender.send((msg, Some(s))).await {
            Err(_) => Err("the mailbox channel is closed send back nothing"),
            Ok(_) => match r.recv().await {
                Err(_) => Err("the response channel is closed send back nothing"),
                Ok(reply_message) => Ok(reply_message),
            },
        }
    }
    pub async fn fire_and_forget(&self, msg:Msg) -> Result<(), SendError<(Msg, Option<Sender<ReplyMsg>>)>> {
        self.message_sender.send((msg, None)).await
    }
}

#[cfg(test)]
mod tests {
    use async_std::task;
    use super::*;
    use futures::future::{OptionFuture};


    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[async_std::test]
    async fn mailbox_processor_tests() {

        enum SendMessageTypes {
            One(i32),
            Two(String),
        }

        #[derive(Debug)]
        enum ResponseMessageTypes {
            Three(i32),
            Four(String),
        }

        let mb = MailboxProcessor::<SendMessageTypes, ResponseMessageTypes>::new( 
            BufferSize::Default, 
            0,  
            |msg, state, reply_channel| async move {
                //if state % 10_000 == 0 {println!("state = {}", state)}
                println!("state = {}", state);
                match msg {
                    SendMessageTypes::One(x) => {
                        println!("hello");
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(ResponseMessageTypes::Three(25 + x)).await.unwrap()
                        })).await;
                    },
                    SendMessageTypes::Two(x) => {
                        println!("yo");
                        let something: i32 = 36;
                        OptionFuture::from(reply_channel.map(|rc| async move {
                            rc.send(ResponseMessageTypes::Four(format!("{}{}", x, something.to_string()))).await.unwrap()
                        })).await;
                    },
                };
                state + 1
            }
        ).await;

        println!("what is the output: {:#?}", mb.send(SendMessageTypes::One(55)).await);
        println!("what is the output: {:#?}", mb.send(SendMessageTypes::Two("I'm a string thing".to_string())).await);

        mb.fire_and_forget(SendMessageTypes::One(55)).await.unwrap();
        mb.fire_and_forget(SendMessageTypes::Two("I'm a string thing".to_string())).await.unwrap();

        //loop {
        //    mb.fire_and_forget(SendMessageTypes::One(55)).await.unwrap();
        //}

        task::sleep(std::time::Duration::from_millis(1000)).await;


//Fn(Msg, State, Option<Sender<ReplyMsg>>
    }



}
