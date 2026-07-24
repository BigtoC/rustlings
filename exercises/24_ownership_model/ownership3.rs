// Module 1 · Ownership — part 3: the borrow checker's core rule.
//
// At any moment a value may be borrowed EITHER by any number of shared
// references (`&T`, read-only) OR by exactly one mutable reference (`&mut T`,
// exclusive) — never both at once. This "aliasing XOR mutability" rule is what
// makes data races impossible at compile time.
//
// Thanks to non-lexical lifetimes (NLL), a borrow ends at its LAST USE, not at
// the end of the enclosing block. So the usual fix for a conflict is to make
// sure you are completely done with the shared borrow before you take the
// exclusive one.

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_a_value_then_grow_the_vec() {
        let mut data = vec![1, 2, 3];

        // TODO: `first` is a SHARED borrow of `data`. `data.push(...)` needs an
        // EXCLUSIVE (mutable) borrow, but `first` is still used on the line
        // after the push, so the shared borrow is still alive there — the two
        // borrows overlap and the code won't compile.
        //
        // Fix it so the shared borrow is finished before the push. The value at
        // index 0 is an `i32` (a `Copy` type), so copy it OUT of `data` (drop
        // the `&`) before pushing: `let first = data[0];`.
        let first = &data[0];
        data.push(*first + 10);
        assert_eq!(*first, 1);

        assert_eq!(data, vec![1, 2, 3, 11]);
    }
}
