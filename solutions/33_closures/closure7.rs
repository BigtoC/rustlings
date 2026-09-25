// Closures - bounds beyond the basics, part 7: a closure bound with `let` is
// not higher-ranked.
//
// Where does a closure's signature come from? When the closure expression sits
// where rustc already EXPECTS a closure type, as the argument of a function with
// an `F: Fn(&str) -> &str` bound or inside a `Box::new(..)` that must become a
// `Box<dyn Fn(&str) -> &str>`, rustc takes the signature from that expectation,
// higher-ranked lifetimes included. A closure bound with `let` has no
// expectation, so rustc infers its signature from the closure alone. The
// annotated `s: &str` parameter gets a fresh lifetime on every call (rustc calls
// it `'1`), but the RETURN type is inferred as one fixed lifetime `'2` that
// cannot depend on `'1`. `s.trim()` borrows from `s`, so the closure body
// already fails: "lifetime may not live long enough ... returning this value
// requires that `'1` must outlive `'2`". Annotating `-> &str` on the closure
// does not help, because closures do not use the `fn` elision rules: that
// `&str` just gets another fresh lifetime. Boxing the closure as a
// `dyn Fn(&str) -> &str` (which is `dyn for<'a> Fn(&'a str) -> &'a str`) then
// also fails with E0308 "one type is more general than the other": the trait
// object needs an output that follows the input's lifetime, and the closure
// only offers one fixed output lifetime.
//
// Only closures behave like this. A named `fn` or a path such as `str::trim`
// always gets the full elided, higher-ranked signature. The direct syntax,
// `for<'a> |s: &'a str| -> &'a str { .. }`, exists but is unstable (E0658,
// tracking issue #97362).
//
// How interviewers probe this: "Why does `let f = |s: &str| s;` fail when the
// same closure passed straight to a function compiles?"

// One stage of a text pipeline: it takes a `&str` and returns a slice of it.
type Stage = Box<dyn Fn(&str) -> &str>;

#[derive(Default)]
struct Pipeline {
    stages: Vec<Stage>,
}

impl Pipeline {
    fn push(&mut self, stage: Stage) {
        self.stages.push(stage);
    }

    // Feeds `input` through every stage in order. The result is a slice of
    // `input`: no stage allocates.
    fn run<'a>(&self, input: &'a str) -> &'a str {
        self.stages.iter().fold(input, |s, stage| stage(s))
    }
}

// Returns its argument unchanged. Its only job is to give a closure an
// expected signature: `F: Fn(&str) -> &str` is `for<'a> Fn(&'a str) -> &'a str`.
fn text_stage<F: Fn(&str) -> &str>(f: F) -> F {
    f
}

// Cleans up a quoted chat line: "  > hello world " becomes "hello".
fn build_pipeline() -> Pipeline {
    // Passed straight to `text_stage`, each closure takes the higher-ranked
    // signature from its bound, so the output borrows from the input. Writing
    // the closures inline in `Box::new(..)`, or as named `fn`s, works too.
    let trim = text_stage(|s| s.trim());
    let first_word = text_stage(|s| s.split_whitespace().next().unwrap_or(s));

    let mut pipeline = Pipeline::default();
    pipeline.push(Box::new(trim));
    // Written inline, so this closure takes its signature from `push`'s
    // parameter type. It already compiles.
    pipeline.push(Box::new(|s| s.strip_prefix('>').unwrap_or(s).trim_start()));
    pipeline.push(Box::new(first_word));
    pipeline
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_a_quoted_line() {
        assert_eq!(build_pipeline().run("  > hello world "), "hello");
    }

    #[test]
    fn unquoted_nested_and_blank_lines() {
        let pipeline = build_pipeline();
        assert_eq!(pipeline.run("plain text"), "plain");
        // Only one level of quoting is removed.
        assert_eq!(pipeline.run(">>nested"), ">nested");
        assert_eq!(pipeline.run("   "), "");
    }

    #[test]
    fn each_stage_does_its_own_job() {
        let pipeline = build_pipeline();
        assert_eq!(pipeline.stages.len(), 3);
        assert_eq!((pipeline.stages[0])("  a b  "), "a b");
        assert_eq!((pipeline.stages[1])(">  quoted text"), "quoted text");
        assert_eq!((pipeline.stages[2])("first second"), "first");
    }

    #[test]
    fn the_result_is_a_slice_of_the_input() {
        let line = String::from("  > hello world ");
        let word = build_pipeline().run(&line);
        assert_eq!(word, "hello");
        // A slice of `line` points into `line`'s own buffer; a copy would sit
        // in a new allocation.
        assert_eq!(word.as_ptr(), line[4..].as_ptr());
    }

    #[test]
    fn one_pipeline_serves_inputs_of_different_lifetimes() {
        let pipeline = build_pipeline();
        let first = {
            let short_lived = String::from(">  short-lived text");
            pipeline.run(&short_lived).to_owned()
        };
        // `pipeline` is still usable after `short_lived` is gone.
        let long_lived = String::from("> long-lived text");
        assert_eq!(first, "short-lived");
        assert_eq!(pipeline.run(&long_lived), "long-lived");
    }

    #[test]
    fn more_stages_can_be_pushed_inline() {
        let mut pipeline = build_pipeline();
        pipeline.push(Box::new(|s| s.trim_end_matches('!')));
        assert_eq!(pipeline.run("> wow!! such"), "wow");
    }
}
