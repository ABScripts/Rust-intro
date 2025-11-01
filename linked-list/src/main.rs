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
        let mut node1 = Node::new(1);
        let node2 = node1.insert(3); // node2 keeps reference to just inserted value which is owned by node1 *
        node2.insert(4);
        node1.insert(2); // * and thus we can't swap this line with the previous one because of ref lifetimes
        // We can't get mut ref for node1 while having active immutable ref
        assert_eq!(
            // No need to implement collect for my iter type - blanket impl would work just fine here
            //
            // Why use "copied"?
            // "copied" is method from Iterator trait which returns iterator which produces copies of original iter items
            // When we iterate using Copied iter we would do: self.it.next().copied();
            // "self.it" is our iter forward. We just call its next method -> return copied value
            //
            // "collect" - one another method from Iterator trait which is used to convert iter -> collection
            // Again, we are good using blanket implementation
            // "collect" will accept Copied iter and use it to create collection, which we specified to be <Vec<_>>
            // <Vec<_>> - that is called "turbofish" and it helps Rust to figure out the collection type we want to get
            // and thus find the right "collect" implementation. We don't specify concrete type of elements in the vector
            // as Rust can compare <Vec<_>> with B returned from "collect" which is trait bounded to B: FromIterator<Self::Item>>.
            // We see that it specifies Self::Item which is already known to the compiler as i32 (taken now from the Copied iter, our
            // original iter features "&i32"
            //
            // Default impl of "collect" calls FromIterator::from_iter(self)
            // Compiler will find "from_iter" implementation which returns turbofished type <Vec<i32>> and call it
            node1.iter_forward().copied().collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn hard() {
        let mut node1 = Node::new(1);
        node1.insert(2).insert(3).insert(4);
        let node2 = node1.clone();
        assert_eq!(
            node1.iter_forward().collect::<Vec<_>>(),
            node2.iter_forward().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn nightmare() {
        // let a = 1;
        // println!("Stack init ptr: {:p}", &a);
        // I used this to print top address of the stack; I also added print in node's drop method
        // First stack address was 0x7f4dc13fe47c and the last printed in drop method 0x7f4dc1200404
        // Diff (0x7f4dc13fe47c−0x7f4dc1200404) = 2_089_080 = ~2MB
        // That is exactly the stack size of thread which run tests under Rust test runner
        // Then we crashed due to stack overflow issue

        let mut node = Node::new(1);

        for index in 0..10_000_000 {
            node.insert(index);
        }
        // We can survive while dropping exactly 16322 items (and stack overflows as it tries dropping 16_323th node)
        // First I figured out that all the values are inserted without any problems - I saw prints for all values up to 10M
        // Then I implemented custom drop, put print there and figured out that the last value dropped was 9_983_678
        // Nodes are stored in the following order (featuring their values below):
        // 1 -> 10M-1 -> 10M-2 -> ... -> 2 -> 1 -> 0
        // Values are deleted from head and the last value deleted was node with value 9_983_678.
        // That means once I have deleted 1 + (9_999_999 - 9_983_678) = 1 + 16321 = 16322 and tried deleting 16_323th node, the stack overflowed

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
