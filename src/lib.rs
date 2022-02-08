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
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
