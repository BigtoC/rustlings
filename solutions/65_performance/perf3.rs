// Module 5 · Performance — part 3: bulk slice operations: check lengths before `zip`, `copy_from_slice`, one `reserve` before `extend_from_slice` (clippy::manual_memcpy).
//
// Index loops are where C habits meet Rust's bounds checks. `a[i]` checks
// `i < a.len()` and panics if not. That check is what makes indexing safe,
// but it is a compare and a branch on every access, and it can keep LLVM
// from vectorizing the loop unless it can prove the check never fails. There
// are three ways to get rid of it without `unsafe`, and each one comes with
// a contract to keep:
//
//   - Iterate instead of indexing. `a.iter().zip(b)` yields pairs and can
//     never go out of bounds, so there is nothing to check. But `zip` STOPS
//     AT THE SHORTER INPUT, silently. For a dot product that is a wrong
//     answer, not a shortcut: decide what unequal lengths mean, and check it
//     before the loop.
//   - Check once, up front. After `assert_eq!(a.len(), b.len())` LLVM can
//     usually prove that every `b[i]` with `i < a.len()` is in bounds, and
//     drop the per-iteration checks. (Re-slicing, `let b = &b[..a.len()];`,
//     gives it the same fact, but does not reject a `b` that is longer.)
//     "Usually" means: confirm it in the assembly (no call to
//     `panic_bounds_check` in the loop), as the `deep-dive/src/perf_lab.rs`
//     lab does. A `debug_assert!` does not count: it is compiled
//     out of release builds, and the contract with it.
//   - Use the bulk operation std already has. `dst.copy_from_slice(src)` is
//     one `memcpy`. It panics unless both slices have the same length, so
//     cut both sides to the length you mean first. `Vec::extend_from_slice`
//     is a `reserve` plus a `memcpy` for `Copy` elements. Clippy's
//     `manual_memcpy` lint flags the element-by-element loop.
//
// Growth is the other hidden cost. A `Vec` that grows by `push` reallocates
// whenever it is full, which can mean copying everything it holds so far.
// With today's doubling strategy, pushing 1_025 bytes one at a time into an
// empty `Vec` calls the allocator 9 times (8, 16, 32, ..., 1_024 and finally
// 2_048 bytes) and leaves half of the last buffer unused. When the final
// size is known, `reserve` it once. Don't count on `extend` or `collect` to
// do it for you after `flat_map` or `filter`: their `size_hint` cannot know
// the total, so the vector grows step by step, much as it does with `push`.
//
// This exercise runs Clippy with `-D warnings` (`strict_clippy`), so every
// warn-by-default lint is an error here.
//
// How interviewers probe this: "What does `zip` do with two slices of
// different lengths?", "How do you get rid of bounds checks without
// `unsafe`?", "Why is `copy_from_slice` faster than a loop, and when does it
// panic?", and "How many times does this `Vec` reallocate?".

// The dot product of two vectors of the same length. Panics with a message
// that contains "length mismatch" if the lengths differ, whichever one is
// longer.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    // The contract first, in every build: `assert_eq!` (unlike
    // `debug_assert_eq!`) stays in release builds, and its message contains
    // "length mismatch" whichever side is longer. Only then `zip`, which
    // stops at the shorter input and so would otherwise hide the mismatch.
    // `zip` never indexes, so there is no bounds check left to eliminate.
    // An index loop behind the same assert would be correct too, and the
    // assert is also what lets LLVM drop its per-iteration bounds checks.
    assert_eq!(a.len(), b.len(), "length mismatch");
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

// Copies the first elements of `src` into the start of `dst`: as many as fit
// in both (the shorter length). Returns how many it copied, and leaves the
// rest of `dst` alone. It never panics.
fn copy_prefix(dst: &mut [i32], src: &[i32]) -> usize {
    // `copy_from_slice` is one `memcpy`, and it panics unless both sides
    // have the same length, so cut both to the shorter length first. Both
    // re-slices are in bounds by construction, so nothing here can panic.
    let n = dst.len().min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

// Appends the chunks to `out`, in order. It assembles a response body from
// many small chunks, so it must grow `out` at most once per call.
fn append_chunks(out: &mut Vec<u8>, chunks: &[&[u8]]) {
    // One `reserve` for the whole total: if `out` lacks room it reallocates
    // once, and if it already has room it does nothing, so the buffer stays
    // where it is. A total of 0 never allocates. Then each
    // `extend_from_slice` is a `memcpy` into space that is already there.
    // (`reserve_exact` works as well; plain `reserve` may round up, to a
    // small minimum or to keep later pushes amortized.)
    let total = chunks.iter().map(|chunk| chunk.len()).sum();
    out.reserve(total);
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // `len` bytes of made-up data.
    fn data(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 7 % 251) as u8).collect()
    }

    // ---- dot ----

    #[test]
    fn dot_of_equal_lengths() {
        assert_eq!(dot(&[1.0, 2.0], &[3.0, 4.0]), 11.0);
        assert_eq!(dot(&[1.5, -2.0, 0.5], &[2.0, 1.0, -4.0]), -1.0);
        assert_eq!(dot(&[], &[]), 0.0);
        let ones = vec![1.0; 10_000];
        let twos = vec![2.0; 10_000];
        assert_eq!(dot(&ones, &twos), 20_000.0);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn dot_panics_when_b_is_shorter() {
        dot(&[1.0, 2.0, 3.0], &[1.0, 2.0]);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn dot_panics_when_a_is_shorter() {
        dot(&[1.0, 2.0], &[1.0, 2.0, 3.0]);
    }

    #[test]
    #[should_panic(expected = "length mismatch")]
    fn dot_panics_when_a_is_empty() {
        dot(&[], &[1.0]);
    }

    // ---- copy_prefix ----

    #[test]
    fn copy_prefix_fills_the_start_of_a_longer_dst() {
        let mut dst = [0; 4];
        assert_eq!(copy_prefix(&mut dst, &[1, 2]), 2);
        assert_eq!(dst, [1, 2, 0, 0]);
    }

    #[test]
    fn copy_prefix_stops_at_the_end_of_dst() {
        let mut dst = [0; 2];
        assert_eq!(copy_prefix(&mut dst, &[7, 8, 9]), 2);
        assert_eq!(dst, [7, 8]);
    }

    #[test]
    fn copy_prefix_edge_cases() {
        let mut dst = [5, 5, 5];
        assert_eq!(copy_prefix(&mut dst, &[]), 0);
        assert_eq!(dst, [5, 5, 5]);

        assert_eq!(copy_prefix(&mut [], &[1, 2, 3]), 0);

        let mut same = [0; 3];
        assert_eq!(copy_prefix(&mut same, &[4, 5, 6]), 3);
        assert_eq!(same, [4, 5, 6]);

        // Into the middle of a bigger buffer, through a sub-slice.
        let mut buffer = vec![0; 10];
        assert_eq!(copy_prefix(&mut buffer[3..6], &[1, 2, 3, 4]), 3);
        assert_eq!(buffer, [0, 0, 0, 1, 2, 3, 0, 0, 0, 0]);

        let big: Vec<i32> = (0..100_000).collect();
        let mut half = vec![-1; 50_000];
        assert_eq!(copy_prefix(&mut half, &big), 50_000);
        assert_eq!(half, big[..50_000]);
    }

    // ---- append_chunks ----

    #[test]
    fn append_chunks_appends_in_order() {
        let mut out = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
        let chunks: [&[u8]; 4] = [b"hello", b"", b", ", b"world"];
        append_chunks(&mut out, &chunks);
        assert_eq!(out, b"HTTP/1.1 200 OK\r\n\r\nhello, world");

        let before = out.clone();
        append_chunks(&mut out, &[]);
        assert_eq!(out, before);
    }

    #[test]
    fn append_chunks_grows_the_buffer_once() {
        // 1_025 bytes: 41 chunks of 25, or 1_025 chunks of one byte. Sized
        // once, the buffer has room for exactly what it holds (the margin
        // below only allows for rounding). Growing step by step, whether byte
        // by byte or chunk by chunk, doubles its way past the total instead.
        let body = data(1_025);
        let limit = 1_025 + 1_025 / 8;
        for size in [25, 1] {
            let chunks: Vec<&[u8]> = body.chunks(size).collect();
            let mut out = Vec::new();
            append_chunks(&mut out, &chunks);
            assert_eq!(out, body);
            assert!(
                out.capacity() <= limit,
                "capacity {} for {} bytes in chunks of {size}: the buffer grew step \
                 by step instead of once",
                out.capacity(),
                out.len()
            );
        }

        // A small buffer that is already full grows once, for everything.
        let chunks: Vec<&[u8]> = body.chunks(25).collect();
        let mut out = vec![0xAA; 7];
        append_chunks(&mut out, &chunks);
        assert_eq!(out.len(), 7 + 1_025);
        assert_eq!(out[7..], body);
        assert!(
            out.capacity() <= limit + 7,
            "capacity {} for {} bytes: the buffer grew step by step instead of once",
            out.capacity(),
            out.len()
        );
    }

    #[test]
    fn append_chunks_keeps_a_buffer_that_has_room() {
        let body = data(1_025);
        let chunks: Vec<&[u8]> = body.chunks(25).collect();
        let mut out = Vec::with_capacity(4_096);
        out.extend_from_slice(b"HTTP/1.1 200 OK\r\n\r\n");
        let heap = out.as_ptr();
        let capacity = out.capacity();

        append_chunks(&mut out, &chunks);
        assert_eq!(out[..19], *b"HTTP/1.1 200 OK\r\n\r\n");
        assert_eq!(out[19..], body);
        assert_eq!(out.as_ptr(), heap, "the buffer moved although it had room");
        assert_eq!(out.capacity(), capacity, "the capacity changed");
    }

    #[test]
    fn appending_nothing_allocates_nothing() {
        let mut out: Vec<u8> = Vec::new();
        append_chunks(&mut out, &[]);
        let empties: [&[u8]; 3] = [b"", b"", b""];
        append_chunks(&mut out, &empties);
        assert!(out.is_empty());
        assert_eq!(
            out.capacity(),
            0,
            "nothing was appended, but the buffer allocated"
        );
    }
}
