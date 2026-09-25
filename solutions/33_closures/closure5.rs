// Closures - bounds beyond the basics, part 5: `Fn(&str)` already means
// `for<'a> Fn(&'a str)`, and why `sort_by_key(|p| &p.name)` fails.
//
// Parts 1-4 used closures whose arguments were plain values. As soon as a
// callback takes a REFERENCE, its bound has to say which lifetimes that
// reference may have, and there are two very different answers.
//
// A lifetime parameter declared on a function, like `'a` in
// `fn f<'a, F: Fn(&'a str) -> usize>(f: F)`, is chosen by the CALLER. Whatever
// the caller picks must stay valid for the whole call, so it always outlives
// every local variable the function body creates. The bound then says "`f`
// accepts `&str`s of that ONE lifetime", and a borrow of a local `String` is
// too short to qualify: E0597 "`greeting` does not live long enough ...
// argument requires that `greeting` is borrowed for `'a`".
//
// What a callback-taking function almost always means is "`f` accepts a `&str`
// of ANY lifetime, however short". That is a higher-ranked trait bound (HRTB):
// `F: for<'a> Fn(&'a str) -> usize`, read "for every lifetime `'a`, `F`
// implements `Fn(&'a str) -> usize`". The binder moves the choice of `'a` from
// the caller to each individual call, so the body may lend `f` a borrow that
// ends one line later. You rarely write `for<'a>` yourself: lifetime elision
// in the `Fn(..)` sugar produces exactly this bound, as it does for `fn`
// pointer types. `Fn(&str) -> usize` IS `for<'a> Fn(&'a str) -> usize`, and
// `Fn(&str) -> &str` is `for<'a> Fn(&'a str) -> &'a str`. The explicit binder
// is needed where elision does not reach, such as a `where` clause on a
// borrowed type (`closure6`).
//
// Part B is the same machinery seen from the other side, and it is the version
// people hit in live coding. `sort_by_key` on slices is declared as
//
//     pub fn sort_by_key<K, F>(&mut self, f: F)
//     where
//         F: FnMut(&T) -> K,
//         K: Ord,
//
// so its bound is `for<'a> FnMut(&'a T) -> K`. The binder covers the argument,
// but `K` is a type parameter of `sort_by_key`: it is picked ONCE, outside the
// binder, so it cannot mention the `'a` of an individual call. A key closure
// that returns `&p.name` would need `K = &'a String` for a different `'a` on
// every call. rustc rejects it with "lifetime may not live long enough ...
// returning this value requires that `'1` must outlive `'2`", a borrowck error
// without an error code: `'1` is the argument's lifetime, `'2` the one lifetime
// baked into `K`. (std could not write `-> K<'a>` instead: a type parameter
// cannot have a lifetime "hole" in it.) `sort_by`'s comparator,
// `FnMut(&T, &T) -> Ordering`, returns an owned `Ordering` that borrows
// nothing, so it may look at borrowed names freely. `sort_by_key(|p| p.age)`
// is fine too: a `u32` key is a copy, not a borrow.
//
// How interviewers probe this: "Read `F: Fn(&str) -> usize` aloud with its
// lifetimes", "Why can't `fn f<'a, F: Fn(&'a str)>` call `f` on a local
// `String`?", and the sharpest one: "Why does `people.sort_by_key(|p| &p.name)`
// fail while `people.sort_by(|a, b| a.name.cmp(&b.name))` compiles?".

// ---- Part A — a callback that borrows a local ------------------------------

// Builds a greeting in a local buffer and lets `measure` inspect it.
fn measure_greeting<F: Fn(&str) -> usize>(name: &str, measure: F) -> usize {
    // `Fn(&str) -> usize` is `for<'a> Fn(&'a str) -> usize`: the callee picks
    // the lifetime at each call, so a borrow of this local is good enough.
    let greeting = format!("hello {name}");
    measure(&greeting)
}

// ---- Part B — sorting by a borrowed key -------------------------------------

// Deliberately NOT `Clone`.
#[derive(Debug, PartialEq)]
struct Person {
    name: String,
    age: u32,
}

// Sorts `people` by name. People with equal names keep their relative order.
fn sort_by_name(people: &mut [Person]) {
    // A comparator returns an owned `Ordering`, so both borrows of the names
    // end inside each call. `sort_by` is stable, like `sort_by_key`.
    people.sort_by(|a, b| a.name.cmp(&b.name));
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Part A ----

    #[test]
    fn measures_the_greeting_length() {
        assert_eq!(measure_greeting("world", |s| s.len()), 11);
    }

    #[test]
    fn counts_the_words_of_the_greeting() {
        assert_eq!(
            measure_greeting("world", |s| s.split_whitespace().count()),
            2
        );
    }

    #[test]
    fn accepts_a_plain_function() {
        // A function path is always higher-ranked: `str::len` implements
        // `for<'a> Fn(&'a str) -> usize`.
        assert_eq!(measure_greeting("Ferris", str::len), 12);
    }

    #[test]
    fn the_callback_sees_the_whole_greeting() {
        let matches = measure_greeting("Ferris", |s| usize::from(s == "hello Ferris"));
        assert_eq!(matches, 1);
    }

    // ---- Part B ----

    fn person(name: &str, age: u32) -> Person {
        Person {
            name: name.to_string(),
            age,
        }
    }

    fn names(people: &[Person]) -> Vec<&str> {
        people.iter().map(|p| p.name.as_str()).collect()
    }

    // The ages of everyone called `name`, in roster order.
    fn ages_named(people: &[Person], name: &str) -> Vec<u32> {
        people
            .iter()
            .filter(|p| p.name == name)
            .map(|p| p.age)
            .collect()
    }

    #[test]
    fn sorts_names_in_byte_order() {
        let mut people = vec![
            person("carol", 30),
            person("Bob", 25),
            person("alice", 41),
            person("Alice", 19),
        ];
        sort_by_name(&mut people);
        // Byte-wise, every uppercase ASCII letter sorts before every lowercase
        // one, so a case-insensitive sort fails here.
        assert_eq!(names(&people), ["Alice", "Bob", "alice", "carol"]);
        // Whole people moved, not just their names.
        assert_eq!(people[0], person("Alice", 19));
        assert_eq!(people[3], person("carol", 30));
    }

    #[test]
    fn empty_and_single_rosters_are_fine() {
        let mut nobody: Vec<Person> = Vec::new();
        sort_by_name(&mut nobody);
        assert!(nobody.is_empty());

        let mut one = vec![person("Solo", 1)];
        sort_by_name(&mut one);
        assert_eq!(one, [person("Solo", 1)]);
    }

    #[test]
    fn equal_names_keep_their_order() {
        // `sort_by_key` is a stable sort, and so must its replacement be. The
        // roster is long enough that an unstable sort visibly reorders people
        // with equal names (on short slices std's unstable sort happens to
        // keep them in order), and the ages are shuffled, so breaking ties by
        // age does not bring back the original order either.
        let pool = ["Mia", "Bob", "Zoe", "Ann"];
        let mut people: Vec<Person> = (0..64)
            .map(|i: usize| person(pool[(i * 7 + i / 3) % pool.len()], (i as u32 * 37) % 64))
            .collect();
        let before: Vec<Vec<u32>> = pool.iter().map(|name| ages_named(&people, name)).collect();
        sort_by_name(&mut people);
        assert!(names(&people).is_sorted());
        for (name, ages) in pool.iter().zip(&before) {
            assert_eq!(
                &ages_named(&people, name),
                ages,
                "people named {name} were reordered"
            );
        }
    }
}
