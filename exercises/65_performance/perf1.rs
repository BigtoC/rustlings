// Module 5 · Performance — part 1: reuse the caller's buffer: `clear()` and `write!` instead of a new `String` per call (clippy::format_push_string).
//
// "This function runs once per request (or per frame, or per log line), and
// it allocates every time. How do you make it stop?" That is the most common
// Rust performance question, and the answer is rarely a faster algorithm. It
// is fewer trips to the allocator. A `String` that starts empty and grows
// line by line calls the allocator again each time it fills up, and may copy
// everything it holds so far. Every `format!` and every `.to_string()` is
// one more allocation, freed a moment later. In a hot loop, that is most of
// the cost.
//
// The standard fix has two halves:
//
//   1. Let the CALLER own the buffer and lend it to you as a `&mut String`
//      (or a `&mut Vec<T>`). `clear()` sets the length to 0 and KEEPS the
//      capacity. A `Vec` (and so a `String`) never shrinks on its own, and
//      its docs promise that emptying it and filling it back up to the same
//      length "should incur no calls to the allocator". After the first call
//      the buffer is big enough, and later calls allocate nothing. std itself
//      works this way: `BufRead::read_line(&mut buf)` appends a line to a
//      buffer you clear and reuse, instead of returning a new `String` per
//      line.
//   2. Format straight INTO that buffer. `String` implements `fmt::Write`, so
//      `write!(out, ..)` and `writeln!(out, ..)` run the formatter directly on
//      the buffer. `out.push_str(&format!(..))` (or `*out += &format!(..)`)
//      first builds a whole temporary `String`, then copies it over and frees
//      it: one allocation per line, for nothing. Clippy's
//      `format_push_string` lint flags exactly that. It is in the `pedantic`
//      group, which is allow-by-default, so it is off unless you ask for it,
//      and this file asks for it.
//
// Assigning a fresh `String` to `*out` looks harmless, but it throws the
// caller's buffer away on every call. The tests catch that with the buffer's
// heap pointer: a new `String` is allocated while the old one is still
// alive, so it cannot have the same address. After one warm-up call,
// `as_ptr()` and `capacity()` must stay exactly the same for a hundred calls.
//
// This exercise runs Clippy with `-D warnings` (`strict_clippy`), so every
// warn-by-default lint is an error here too. One of them,
// `format_in_format_args`, catches the same temporary handed to `write!` as
// an argument: `write!(out, "{}", format!(..))`.
//
// How interviewers probe this: "Where are the allocations in this loop?",
// "Why take a `&mut String` instead of returning a `String`?", "What does
// `clear()` do to the capacity?", and "Why is `write!` into a `String` better
// than `push_str(&format!(..))`, and when can it return an error?".

// Turns on the allow-by-default lint for this file. Keep it.
#![deny(clippy::format_push_string)]

// One line of a stock dashboard.
struct Row {
    item: String,
    count: u32,
}

// The dashboard as text: a header line, one line per row, and the total.
// Each line is the item left-aligned in 10 columns, then the number
// right-aligned in 7:
//
//   item        count
//   apples          3
//   pears          12
//   total          15
fn render(rows: &[Row]) -> String {
    // TODO: Clippy rejects the three `+= &format!(..)` lines below with
    // `format_push_string` ("`format!(..)` appended to existing `String`"):
    // each one builds a temporary `String`, copies it into `table` and frees
    // it. Requirements:
    //   - write every line straight into the destination buffer, with no
    //     temporary `String` per line. Handing the `format!` result to
    //     `write!` is the same temporary (`format_in_format_args`), and so is
    //     binding it to a variable first, which Clippy does not flag;
    //   - `render` keeps its signature and returns exactly the same text;
    //   - keep the `#![deny]` above, and add no `#[allow(..)]`.
    // Until you format into the buffer itself, this exercise will not pass.
    let mut table = String::new();
    table += &format!("{:<10}{:>7}\n", "item", "count");
    let mut total: u64 = 0;
    for row in rows {
        table += &format!("{:<10}{:>7}\n", row.item, row.count);
        total += u64::from(row.count);
    }
    table += &format!("{:<10}{:>7}\n", "total", total);
    table
}

// The dashboard loop calls this once per frame, always with the same buffer.
// Afterwards `out` holds exactly the table for `rows`, whatever it held
// before.
fn render_into(rows: &[Row], out: &mut String) {
    // TODO: `render_into_reuses_the_callers_buffer` fails with "frame 0:
    // `out` points at a different heap buffer": this line allocates a whole
    // new `String` and drops the caller's buffer, capacity and all, on every
    // call. Requirements:
    //   - after the call `out` holds exactly the table for `rows` (nothing
    //     left over from before);
    //   - when `out` already has enough capacity, its heap pointer and its
    //     capacity stay the same: no new `String`, no `shrink_to_fit`, no
    //     swapping in another buffer;
    //   - no temporary `String` either: building the table somewhere else
    //     and copying it into `out` passes the pointer test but still
    //     allocates on every call;
    //   - keep both signatures, and keep a single copy of the layout code.
    // Until you reuse `out` instead of replacing it, the tests will fail.
    *out = render(rows);
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(items: &[(&str, u32)]) -> Vec<Row> {
        items
            .iter()
            .map(|&(item, count)| Row {
                item: item.to_string(),
                count,
            })
            .collect()
    }

    // `count` rows named "item-00", "item-01", ... with made-up counts.
    fn numbered_rows(count: u32) -> Vec<Row> {
        (0..count)
            .map(|i| Row {
                item: format!("item-{i:02}"),
                count: i * 37 % 1_000,
            })
            .collect()
    }

    // The expected table, built line by line from the same layout rule.
    fn expected(rows: &[Row]) -> String {
        let mut lines = vec![format!("{:<10}{:>7}\n", "item", "count")];
        let mut total: u64 = 0;
        for row in rows {
            lines.push(format!("{:<10}{:>7}\n", row.item, row.count));
            total += u64::from(row.count);
        }
        lines.push(format!("{:<10}{:>7}\n", "total", total));
        lines.concat()
    }

    #[test]
    fn render_builds_the_table() {
        let table = render(&rows(&[("apples", 3), ("pears", 12)]));
        assert_eq!(
            table,
            "item        count\n\
             apples          3\n\
             pears          12\n\
             total          15\n"
        );

        let empty = render(&[]);
        assert_eq!(empty, "item        count\ntotal           0\n");

        // The total is a `u64`, so two big counts do not overflow; a number
        // wider than its column simply makes the line longer.
        let big = render(&rows(&[("a", u32::MAX), ("b", u32::MAX)]));
        assert_eq!(
            big,
            "item        count\n\
             a         4294967295\n\
             b         4294967295\n\
             total     8589934590\n"
        );
    }

    #[test]
    fn render_into_replaces_what_the_buffer_held() {
        let board = rows(&[("apples", 3), ("pears", 12)]);

        // Old content that is longer than the table.
        let mut buf = "stale line\n".repeat(50);
        render_into(&board, &mut buf);
        assert_eq!(buf, expected(&board));

        // And shorter.
        let mut buf = String::from("x");
        render_into(&board, &mut buf);
        assert_eq!(buf, expected(&board));

        // The same rows twice give the same text, not the table twice.
        render_into(&board, &mut buf);
        assert_eq!(buf, expected(&board));
    }

    #[test]
    fn render_into_reuses_the_callers_buffer() {
        // The frames the loop cycles through; the first one is the biggest.
        let frames = [
            numbered_rows(40),
            numbered_rows(3),
            Vec::new(),
            numbered_rows(25),
            numbered_rows(40),
        ];
        let tables: Vec<String> = frames.iter().map(|frame| expected(frame)).collect();

        // Warm-up: the first call sizes the buffer.
        let mut buf = String::new();
        render_into(&frames[0], &mut buf);
        assert_eq!(buf, tables[0]);
        let heap = buf.as_ptr();
        let capacity = buf.capacity();

        for (frame, (rows, table)) in frames.iter().zip(&tables).cycle().take(100).enumerate() {
            render_into(rows, &mut buf);
            assert_eq!(buf, *table, "frame {frame}: wrong text");
            assert_eq!(
                buf.as_ptr(),
                heap,
                "frame {frame}: `out` points at a different heap buffer, so the \
                 caller's buffer was replaced instead of reused"
            );
            assert_eq!(
                buf.capacity(),
                capacity,
                "frame {frame}: the capacity changed"
            );
        }
    }

    #[test]
    fn render_into_grows_an_empty_buffer_then_keeps_it() {
        let board = numbered_rows(12);
        let mut buf = String::new();
        render_into(&board, &mut buf);
        assert_eq!(buf, expected(&board));
        let heap = buf.as_ptr();
        let capacity = buf.capacity();

        render_into(&board, &mut buf);
        assert_eq!(buf, expected(&board));
        assert_eq!(buf.as_ptr(), heap, "the second call replaced the buffer");
        assert_eq!(buf.capacity(), capacity);

        // A smaller frame keeps it too, and so does an empty dashboard.
        render_into(&board[..1], &mut buf);
        assert_eq!(buf, expected(&board[..1]));
        render_into(&[], &mut buf);
        assert_eq!(buf, "item        count\ntotal           0\n");
        assert_eq!(buf.as_ptr(), heap, "a smaller frame replaced the buffer");
        assert_eq!(buf.capacity(), capacity);
    }
}
