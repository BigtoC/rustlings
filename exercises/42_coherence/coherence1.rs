// Traits & Abstraction · Coherence — part 1: the orphan rule, newtypes, and `From` for a foreign type (E0117, E0277).
//
// Coherence is the promise that for any type and any trait (with its generic
// arguments: `From<u8>` and `From<u16>` count as two traits) there is AT MOST
// ONE impl in the whole program: your crate plus every crate it depends on.
// Method calls, trait bounds and `{}` all need a single answer to "which
// `fmt` runs here?", and every crate that uses the same trait and type must
// get the same answer. rustc keeps the promise with two checks. The overlap
// check (part 2) rejects an impl that could apply to the same type as another
// impl, whether that one is in your crate or in a dependency. The ORPHAN
// RULE, this part, decides where an impl may be written at all, so that two
// crates that don't know about each other can never both write it.
//
// The rule, from the Reference: for `impl<P..> Trait<T1, .., Tn> for T0`,
// either `Trait` is defined in this crate, or at least one of the types
// `T0..=Tn` is local, with no uncovered type parameter (a bare `P`, not
// wrapped in some other type; `&P` and `Box<P>` count as bare, part 2 says
// why) appearing before the first local one. A type is local when its
// OUTERMOST constructor is defined in this crate: `Point` is local,
// `Vec<Point>` is not, however local its element type is. A `type` alias
// changes nothing, because an alias is only a second name for the same type.
//
// Why so strict when no other crate even knows `Point`? Because std may add
// an impl such as `impl<T: Debug> Display for Vec<T>` in any future release.
// The orphan rule is what makes that a compatible change for std: if you were
// allowed to write `impl Display for Vec<Point>`, the new std impl would
// overlap with yours and your crate would stop compiling. rustc says E0117
// "only traits defined in the current crate can be implemented for types
// defined outside of the crate", and its last note gives you the two ways
// out: "define and implement a trait or new type instead".
//
// A newtype is a struct with one field, the foreign type, and it IS local. It
// costs nothing at run time: it has the same size as the value it wraps
// (`#[repr(transparent)]` makes that a documented guarantee), and wrapping
// and unwrapping are moves. The price is paid in API: the wrapper has none of
// the inner type's traits and methods until you add them. Here the tests
// `collect()` into it, so it needs `FromIterator`. (`45_sized_deref` covers
// `Deref`, the other way to pass an API through, in `deref1`, and when it is
// the wrong tool, in `deref2`.)
//
// Part B is the half interviewers use as a follow-up: "so can you ever
// implement a foreign trait for a foreign type?". Yes, when a local type
// appears in the TRAIT's parameters. In `impl From<Point> for (i32, i32)`,
// `From` and the tuple are both std's, but `T1 = Point` is local and there is
// no type parameter at all, so the impl is legal. Before Rust 1.41 the
// generic version (`impl<T> From<Wrapper<T>> for Vec<T>`) was rejected, and a
// lot of code learned to write `impl Into<..> for Local` instead. That habit
// is what the starter below shows, and it costs callers something: core's
// blanket `impl<T, U: From<T>> Into<U> for T` gives you `Into` for free once
// `From` exists, but an `Into` impl gives you no `From` at all.
//
// How interviewers probe this: "Why can't you write `impl Display for
// Vec<Point>`, and what are your options?", "Why doesn't a type alias help?",
// "Is `impl From<MyType> for String` legal?", "`From` or `Into`, which do you
// implement?".

use std::fmt;

// Deliberately plain: `Point` itself is local, so it may implement anything.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// ---- Part A — printing a list of points -----------------------------------

// TODO: rustc rejects this impl with E0117 "only traits defined in the current
// crate can be implemented for types defined outside of the crate": `Display`
// belongs to std, and so does `Vec`, whatever its element type.
// Requirements: the tests want a type named `Polyline`. They build one
// straight from a `Vec<Point>` as `Polyline(points)`, and by `collect()`ing any
// iterator of `Point`s. It must print with `{}` exactly as this impl does (the
// points joined by " -> ", and nothing at all for an empty line), and it must
// be no bigger than the `Vec` it holds. Keep `Point` as it is. A `type` alias
// won't help (aliases don't make a type local), and neither will a local
// trait with its own `to_string`: the tests use `{}` and std's `to_string`.
// Don't edit the tests. Until `Polyline` exists and can be both collected and
// printed, this exercise will not compile.
impl fmt::Display for Vec<Point> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, point) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(" -> ")?;
            }
            write!(f, "{point}")?;
        }
        Ok(())
    }
}

// ---- Part B — a `Point` as a plain pair ----------------------------------

// Given, complete: generic code over "anything that converts into a pair",
// bounded on `From`, the trait an API is meant to implement.
fn to_pairs<T>(items: Vec<T>) -> Vec<(i32, i32)>
where
    (i32, i32): From<T>,
{
    items.into_iter().map(<(i32, i32)>::from).collect()
}

// TODO: the tests call `<(i32, i32)>::from(point)` and `to_pairs(points)`, and
// both fail with E0277 "the trait bound `(i32, i32): From<Point>` is not
// satisfied": an `Into` impl gives you `.into()` and nothing else. Its author
// wrote `Into` because `From<Point> for (i32, i32)` looked like a foreign
// trait for a foreign type. Check that against the rule in the header.
// Requirements: `From::from`, `.into()` and `to_pairs` all work for `Point`,
// the target stays a plain `(i32, i32)`, and there is no helper function such
// as `fn to_pair(p: Point)`. (If you keep this `Into` impl next to your new
// one, read the E0119 you get: it names the impl in `core` that makes it
// redundant.) Until `(i32, i32): From<Point>` holds, this exercise will not
// compile.
impl Into<(i32, i32)> for Point {
    fn into(self) -> (i32, i32) {
        (self.x, self.y)
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    fn p(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    // ---- Part A ----

    #[test]
    fn prints_points_joined_by_arrows() {
        let line: Polyline = [p(0, 0), p(1, 2)].into_iter().collect();
        assert_eq!(line.to_string(), "(0, 0) -> (1, 2)");
        assert_eq!(format!("[{line}]"), "[(0, 0) -> (1, 2)]");
    }

    #[test]
    fn an_empty_line_prints_nothing() {
        let collected: Polyline = std::iter::empty().collect();
        assert_eq!(collected.to_string(), "");
        assert_eq!(Polyline(Vec::new()).to_string(), "");
    }

    #[test]
    fn a_single_point_has_no_arrow() {
        assert_eq!(Polyline(vec![p(3, -4)]).to_string(), "(3, -4)");
    }

    #[test]
    fn collects_from_any_iterator_of_points() {
        // Not just arrays and `Vec`s: a lazy `map` over a range works too,
        // because `collect` only needs `FromIterator<Point>`.
        let parabola: Polyline = (0..4).map(|i| p(i, i * i)).collect();
        assert_eq!(parabola.0.len(), 4);
        assert_eq!(parabola.to_string(), "(0, 0) -> (1, 1) -> (2, 4) -> (3, 9)");
    }

    #[test]
    fn wraps_the_callers_vec_without_copying_it() {
        let points = vec![p(1, 1), p(2, 2), p(3, 3)];
        let heap = points.as_ptr();
        let line = Polyline(points);
        // Wrapping is a move of the `Vec` header; the points stay where
        // they are.
        assert_eq!(line.0.as_ptr(), heap);
        assert_eq!(line.to_string(), "(1, 1) -> (2, 2) -> (3, 3)");
    }

    #[test]
    fn the_newtype_costs_nothing() {
        // One field, no extra state: the wrapper is exactly as big as the
        // `Vec` it holds.
        assert_eq!(size_of::<Polyline>(), size_of::<Vec<Point>>());
    }

    // ---- Part B ----

    #[test]
    fn a_point_converts_with_from_and_into() {
        assert_eq!(<(i32, i32)>::from(p(-1, 0)), (-1, 0));
        let pair: (i32, i32) = p(3, 4).into();
        assert_eq!(pair, (3, 4));
    }

    #[test]
    fn generic_code_bounded_on_from_accepts_points() {
        assert_eq!(to_pairs(vec![p(1, 2), p(3, 4)]), [(1, 2), (3, 4)]);
        assert_eq!(to_pairs(Vec::<Point>::new()), []);
        // The same function still takes plain pairs, through core's
        // reflexive `impl<T> From<T> for T`.
        assert_eq!(to_pairs(vec![(5, 6)]), [(5, 6)]);
    }
}
