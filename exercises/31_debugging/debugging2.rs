// Module 5 · Debugging — part 2: integer overflow behaves differently per build.
//
// Rust does NOT silently ignore integer overflow. What it does depends on the
// build profile:
//   - debug builds (with `debug_assertions` on) insert overflow checks, so an
//     overflowing `+` PANICS with "attempt to add with overflow".
//   - release builds (`--release`, checks off) let the value WRAP around modulo
//     2^bits — `u32::MAX + 1` becomes `0`.
// Either way the naive `acc += v` gives a wrong answer on large inputs; the
// difference is only whether you find out with a panic or a silently bad number.
//
// The fix is to pick the behavior ON PURPOSE with an explicit method:
//   - `checked_add(v)`     -> `Option`, `None` on overflow (handle it)
//   - `saturating_add(v)`  -> clamps at `u32::MAX` / `u32::MIN`
//   - `wrapping_add(v)`    -> modular arithmetic, wrap deliberately
// `checksum` here should CLAMP: once we hit the ceiling, stay there.

// TODO: `acc += v` overflows once the running total passes `u32::MAX`. In debug
// builds the large-input test panics; in release it would wrap to a small wrong
// number. Decide the intended behavior — we want to clamp at the maximum — and
// replace `acc += v;` with `acc = acc.saturating_add(v);`.
fn checksum(values: &[u32]) -> u32 {
    let mut acc = 0u32;
    for &v in values {
        acc += v;
    }
    acc
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_input_sums_normally() {
        assert_eq!(checksum(&[1, 2, 3]), 6);
    }

    #[test]
    fn large_input_saturates() {
        // The running total blows past `u32::MAX`; clamping keeps it pinned
        // there instead of panicking (debug) or wrapping to a small value.
        assert_eq!(checksum(&[u32::MAX, 10, 10]), u32::MAX);
    }
}
