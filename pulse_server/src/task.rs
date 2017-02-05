
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct TaskHandle<T, C> {
    pub data_receiver: Receiver<T>,
    pub command_sender: Sender<C>,
}

pub struct Task<T, C> {
    pub data_sender: Sender<T>,
    pub command_receiver: Receiver<C>,
}

pub fn init_task<T, C>() -> (Task<T, C>, TaskHandle<T, C>) {
    let (data_sender, data_receiver) = channel();
    let (command_sender, command_receiver) = channel();

    let task = Task {
        data_sender: data_sender,
        command_receiver: command_receiver,
    };

    let task_handle = TaskHandle {
        data_receiver: data_receiver,
        command_sender: command_sender,
    };

    (task, task_handle)
}
