mod node;

#[cfg(test)]
mod linked_list_tests {
    use crate::node::Node;

    #[test]
    fn trivial() {
        let mut node = Node::new(1);
        node.insert(2).insert(3).insert(4);
        println!("{node}");
        assert_eq!(node.to_string(), "1,2,3,4");
    }

    #[test]
    fn easy() {
        let mut node = Node::new(42);
        assert_eq!(**node, 42);
        **node = 13;
        assert_eq!(**node, 13);
    }

    #[test]
    fn normal() {
        // let mut node1 = Node::new(1);
        // let mut node2 = node1.insert(3);
        // node1.insert(2);
        // node2.insert(4);
        // assert_eq!(node1.iter_forward().collect(), [1, 2, 3, 4]);
    }

    #[test]
    fn hard() {
        // let mut node1 = Node::new(1);
        // node1.insert(2).insert(3).insert(4);
        // let node2 = node1.clone();
        // assert_eq!(
        //     node1.iter_forward().collect(),
        //     node2.iter_forward().collect(),
        // );
    }

    #[test]
    fn nightmare() {
        // let mut node = Node::new(1);
        // for index in 0..10_000_000 {
        //     node.insert(index);
        // }
        // did it panic ??
    }

    #[test]
    fn ultra_nightmare() {
        // let mut node = Node::new(1);
        // node.insert(2).insert(3).insert(4);
        // let last = node.iter_forward().last().unwrap();
        // assert_eq!(last.iter_bacwards().collect(), [4, 3, 2, 1]);
    }
}

fn main() {}
