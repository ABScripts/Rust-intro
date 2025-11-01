use std::clone;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::ops::{Deref, DerefMut};

use crate::node;

pub mod node_forward_iter;
#[derive(Debug)]
pub struct Node<T: Debug + Clone> {
    value: T,
    next: Option<Box<Node<T>>>,
    // do I need to do something about this "next" here?
}

impl<T: Debug + Clone> Node<T> {
    /// creates new node with a value
    pub fn new(value: T) -> Box<Self> {
        Box::new(Node {
            value,
            next: Option::None,
        })
    }

    /// creates new node with a value and inserts it after this one
    pub fn insert(&mut self, value: T) -> &mut Node<T> {
        let Some(_) = &self.next else {
            return self.next.insert(Node::new(value));
        };

        let mut new_node = Node::new(value);
        new_node.next = self.next.take();

        // this "insert" can be confusing here but it is called for the Option
        // so there it no recursion
        self.next.insert(new_node);

        // let a = 1;
        // println!("{} a: {:p}", val, &a); // this value always has constant address and seems that stack doesn't move here
        // ok, so this method actually executes till the end and then fails
        // maybe it has something to do with drop??

        return self.next.as_deref_mut().unwrap();
    }

    /// iterates all nodes starting with this one and forward
    pub fn iter_forward(&self) -> node_forward_iter::NodeForwardIter<'_, T> {
        node_forward_iter::NodeForwardIter {
            current: Some(self),
        }
    }
}

impl<T: Debug + Display + Clone> Display for Node<T> {
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

// why does specifying Display + Clone for "T" throws compiler error??
// because we can't specialize "Drop" impl for the type - it MUST always be single implementation
impl<T: Debug + Clone> Drop for Node<T> {
    fn drop(&mut self) {
        // println!("Dropping {:?}", self.value);   // last value dropped here was 9_983_678

        // let a = 1;
        // println!("{:p}", &a);  <-- shows that stack increases as drop is called recursively

        // do I need to clean it from the end??
        // then it wouldn't go into infinite recursion? nope I guess..
        // actually I would use list above then I wouldn't have a problem that I need to implement common dropper
        // as list would be cleaned by itearting over nodes and deleting just one node at a time
        // here node is itself a list, kind of..

        let mut preserved_node = self.next.take();
        // drop(self);
        // ^^^ we don't need this; first orphaned node will be dropped automatically at the end of this method
        while let Some(mut next_node) = preserved_node {
            preserved_node = next_node.next.take(); // orphan "next_node", preserved_node keeps reference to the next node
            // drop(next_node); // will automatically be dropped here; next_node is newly created on each iteration
        }
    }
}

impl<T: Debug + Clone> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Debug + Clone> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// "T" MUST implement Clone trait as we try cloning value here which is of type "T"
impl<T: Clone + Display + Debug> Clone for Node<T> {
    fn clone(&self) -> Self {
        // use clone on the value as we don't want to move it and most certainly it won't be copiable
        let mut cloned_head_node = Node::new(self.value.clone());

        // start looping from the second node as we already copied the first just above ^^^
        // "skip" returns new iterator which skips first N elements (1 in this case)
        // on the first call to "next" it will return value of (N + 1) element or None
        // let mut iter = self.iter_forward().skip(1);
        let mut last_node_ref = &mut *cloned_head_node;
        for value in self.iter_forward().skip(1) {
            last_node_ref = last_node_ref.insert(value.clone());
        }

        // data will be moved from heap to stack
        *cloned_head_node
    }
}
