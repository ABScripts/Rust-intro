use super::Node;

pub struct NodeForwardIter<'a, T> {
    pub current: Option<&'a Node<T>>,
}

impl<'a, T> Iterator for NodeForwardIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };

        let value = &current.value;

        self.current = current.next.as_deref();

        Some(value)
    }
}
