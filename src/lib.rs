use async_std::prelude::*;
use async_std::channel::*;
use async_std::task;
use std::fmt::Display;
use thiserror::Error;

pub struct MailboxProcessor<Msg, ReplyMsg> {
    message_sender: Sender<(Msg, Option<Sender<ReplyMsg>>)>,
}

pub enum BufferSize {
    Default,
    Size(usize),
}

#[derive(Debug, Error)]
pub struct MailboxProcessorError {
    msg: String,
    #[source]
    source: Option<Box<dyn std::error::Error + std::marker::Send + Sync + 'static>>,
}
impl Display for MailboxProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg) } }

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
    pub async fn send(&self, msg:Msg) -> Result<ReplyMsg, MailboxProcessorError> {
        let (s, r) = bounded(1);
        match self.message_sender.send((msg, Some(s))).await {
            Err(_) => Err(MailboxProcessorError { msg: "the mailbox channel is closed send back nothing".to_owned(), source: None}),
            Ok(_) => match r.recv().await {
                Err(_) => Err(MailboxProcessorError { msg: "the response channel is closed (did you mean to call fire_and_forget() rather than send())".to_owned(), source: None}),
                Ok(reply_message) => Ok(reply_message),
            },
        }
    }
    //pub async fn fire_and_forget(&self, msg:Msg) -> Result<(), SendError<(Msg, Option<Sender<ReplyMsg>>)>> {
    pub async fn fire_and_forget(&self, msg:Msg) -> Result<(), MailboxProcessorError> {
        self.message_sender.send((msg, None)).await.map_err(|_| MailboxProcessorError {msg: "the mailbox channel is closed send back nothing".to_owned(), source: None})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::{OptionFuture};

    #[async_std::test]
    async fn mailbox_processor_tests() {

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
    }
}
