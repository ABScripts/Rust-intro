use std::fmt::Display;
use std::ops::{Deref, DerefMut};

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    /// creates new node with a value
    fn new(value: T) -> Box<Self> {
        todo!()
    }

    /// creates new node with a value and inserts it after this one
    fn insert(&mut self, value: T) -> &mut Node<T> {
        todo!()
    }

    /// iterates all nodes starting with this one and forward
    fn iter_forward(&self) -> NodeForwardIter<'_, T> {
        todo!()
    }
}

impl<T> Display for Node<T> {
    // ..
}

impl<T> Deref for Node<T> {
    // ..
}

impl<T> DerefMut for Node<T> {
    // ..
}

impl<T: Clone> Clone for Node<T> {
    // ..
}

struct NodeForwardIter<'a, T> {
    // ...
}

impl<'a, T> Iterator for NodeForwardIter<'a, T> {
    // ...
}

fn main() {
    // test 0 - trivial
    {
        let mut node = Node::new(1);
        node.insert(2).insert(3).insert(4);
        println!("{node}"); // should print: 1,2,3,4
        assert_eq!(node.to_string(), "1,2,3,4");
    }

    // test 1 - easy
    {
        let mut node = Node::new(42);
        assert_eq!(**node, 42);
        **node = 13;
        assert_eq!(**node, 13);
    }

    // test 2 - normal
    {
        let mut node1 = Node::new(1);
        let mut node2 = node1.insert(3);
        node1.insert(2);
        node2.insert(4);
        assert_eq!(node1.iter_forward().collect(), [1, 2, 3, 4]);
    }

    // test 3 - hard
    {
        let mut node1 = Node::new(1);
        node1.insert(2).insert(3).insert(4);
        let node2 = node1.clone();
        assert_eq!(
            node1.iter_forward().collect(),
            node2.iter_forward().collect(),
        );
    }

    // test 4 - nightmare
    {
        let mut node = Node::new(1);
        for index in 0..10_000_000 {
            node.insert(index);
        }
        // did it panic ??
    }

    // test 5 - ultra nightmare (requires changes in the provided template)
    {
        let mut node = Node::new(1);
        node.insert(2).insert(3).insert(4);
        let last = node.iter_forward().last().unwrap();
        assert_eq!(last.iter_bacwards().collect(), [4, 3, 2, 1]);
    }
}
