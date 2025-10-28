use super::Node;

pub struct NodeForwardIter<'a, T> {
    current: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for NodeForwardIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!();
    }
}
