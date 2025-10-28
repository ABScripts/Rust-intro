use std::fmt::Display;
use std::ops::{Deref, DerefMut};

pub mod node_forward_iter;

pub struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    /// creates new node with a value
    pub fn new(value: T) -> Box<Self> {
        todo!();
    }

    /// creates new node with a value and inserts it after this one
    pub fn insert(&mut self, value: T) -> &mut Node<T> {
        todo!()
    }

    /// iterates all nodes starting with this one and forward
    pub fn iter_forward(&self) -> node_forward_iter::NodeForwardIter<'_, T> {
        todo!()
    }
}

impl<T: Display> Display for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value: {}, Next: ", self.value)?;
        match &self.next {
            Some(next) => write!(f, "{}", next),
            None => write!(f, "None"),
        }
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        todo!()
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
