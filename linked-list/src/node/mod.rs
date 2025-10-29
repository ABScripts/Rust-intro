use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use crate::node;

pub mod node_forward_iter;

pub struct Node<T> {
    value: T,
    pub next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    /// creates new node with a value
    pub fn new(value: T) -> Box<Self> {
        Box::new(Node {
            value,
            next: Option::None,
        })
    }

    /// creates new node with a value and inserts it after this one
    pub fn insert(&mut self, value: T) -> &mut Node<T> {
        self.next = Some(Node::new(value));
        self.next.as_deref_mut().unwrap()
    }

    /// iterates all nodes starting with this one and forward
    pub fn iter_forward(&self) -> node_forward_iter::NodeForwardIter<'_, T> {
        node_forward_iter::NodeForwardIter {
            current: Some(self),
        }
    }
}

impl<T: Display> Display for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.iter_forward();

        if let Some(value) = iter.next() {
            write!(f, "{}", value)?;
            while let Some(value) = iter.next() {
                write!(f, ",{}", value)?;
            }
        }

        Ok(())
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
