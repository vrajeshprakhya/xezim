//! §20.5, §20.8.2: the real math functions of Table 20-4 and the real
//! conversions compile to `Insn::RealMath`, so a timed process that calls one
//! stays on the process-FSM path. Before, every one of them reached the
//! expression compiler's `SystemCall_other` arm; the statement fallback it
//! produced gated the whole process out of compilation, even when the call
//! sat on a branch that never ran (#1326).
//!
//! Each value is checked bit for bit against Rust's `f64` (the IEEE 754
//! operations Table 20-4 refers to through C's <math.h>), printed as
//! `$realtobits` hex so no decimal formatting sits in between. Each call is
//! an ordinary assignment: inside a `$display` argument it would be the
//! interpreter, not `RealMath`, that evaluated it.
use std::process::Command;

fn run(src: &str) -> String {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "xezim_real_math_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temporary directory");
    let path = dir.join("design.sv");
    std::fs::write(&path, src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_xezim"))
        .args([
            "--simulate",
            "-s",
            "top",
            path.to_str().unwrap(),
            "--no-cache",
        ])
        .env("XEZIM_PROC_LOOP_STATS", "1")
        .env("XEZIM_FALLBACK_SITES", "1")
        .output()
        .expect("run xezim");
    let _ = std::fs::remove_dir_all(&dir);
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "simulation failed:\n{text}");
    text
}

/// The process compiled: registered on the process-FSM path, never gated
/// out, and no system function left for the AST interpreter.
fn assert_compiled(text: &str) {
    assert!(
        text.lines().any(|l| l.starts_with("[PROC-FSM] registered")),
        "the timed process was not compiled:\n{text}"
    );
    assert!(
        !text.lines().any(|l| l.contains("gated out")),
        "a process was gated out of compilation:\n{text}"
    );
    assert!(
        !text.lines().any(|l| l.contains("SystemCall_other")),
        "a system function fell back to the interpreter:\n{text}"
    );
}

const ALL_FUNCTIONS: &str = r#"
`timescale 1ns/1ns
module top;
  real x = 0.3;
  real y = 2.0;
  real r;
  integer k;
  int  n = 0;
  always #(1) begin
    if (n < 4) begin
      r = $sqrt(x);
      $display("R|sqrt|%016h", $realtobits(r));
      r = $ln(x);
      $display("R|ln|%016h", $realtobits(r));
      r = $log10(x);
      $display("R|log10|%016h", $realtobits(r));
      r = $exp(x);
      $display("R|exp|%016h", $realtobits(r));
      r = $pow(x, y);
      $display("R|pow|%016h", $realtobits(r));
      r = $floor(x * 3.7);
      $display("R|floor|%016h", $realtobits(r));
      r = $ceil(x * 3.7);
      $display("R|ceil|%016h", $realtobits(r));
      r = $sin(x);
      $display("R|sin|%016h", $realtobits(r));
      r = $cos(x);
      $display("R|cos|%016h", $realtobits(r));
      r = $tan(x);
      $display("R|tan|%016h", $realtobits(r));
      r = $asin(x);
      $display("R|asin|%016h", $realtobits(r));
      r = $acos(x);
      $display("R|acos|%016h", $realtobits(r));
      r = $atan(x);
      $display("R|atan|%016h", $realtobits(r));
      r = $atan2(x, y);
      $display("R|atan2|%016h", $realtobits(r));
      r = $hypot(x, y);
      $display("R|hypot|%016h", $realtobits(r));
      r = $sinh(x);
      $display("R|sinh|%016h", $realtobits(r));
      r = $cosh(x);
      $display("R|cosh|%016h", $realtobits(r));
      r = $tanh(x);
      $display("R|tanh|%016h", $realtobits(r));
      r = $asinh(x);
      $display("R|asinh|%016h", $realtobits(r));
      r = $acosh(x + 1.0);
      $display("R|acosh|%016h", $realtobits(r));
      r = $atanh(x);
      $display("R|atanh|%016h", $realtobits(r));
      r = $itor(n - 2);
      $display("R|itor|%016h", $realtobits(r));
      r = $bitstoreal($realtobits(x));
      $display("R|bits|%016h", $realtobits(r));
      k = $rtoi(x * 10.0);
      $display("I|rtoi|%0d", k);
    end
    x = x + 0.2;
    n = n + 1;
    if (n == 5) $finish;
  end
endmodule
"#;

fn expected_all() -> Vec<String> {
    let bits = |v: f64| format!("{:016x}", v.to_bits());
    let (mut x, y) = (0.3_f64, 2.0_f64);
    let mut out = Vec::new();
    for n in 0..4_i64 {
        for (name, v) in [
            ("sqrt", x.sqrt()),
            ("ln", x.ln()),
            ("log10", x.log10()),
            ("exp", x.exp()),
            ("pow", x.powf(y)),
            ("floor", (x * 3.7).floor()),
            ("ceil", (x * 3.7).ceil()),
            ("sin", x.sin()),
            ("cos", x.cos()),
            ("tan", x.tan()),
            ("asin", x.asin()),
            ("acos", x.acos()),
            ("atan", x.atan()),
            ("atan2", x.atan2(y)),
            ("hypot", x.hypot(y)),
            ("sinh", x.sinh()),
            ("cosh", x.cosh()),
            ("tanh", x.tanh()),
            ("asinh", x.asinh()),
            ("acosh", (x + 1.0).acosh()),
            ("atanh", x.atanh()),
            // §20.5: a signed integer converts with its sign.
            ("itor", (n - 2) as f64),
            ("bits", x),
        ] {
            out.push(format!("R|{name}|{}", bits(v)));
        }
        // §20.5 / §6.12.2: $rtoi truncates toward zero.
        out.push(format!("I|rtoi|{}", (x * 10.0).trunc() as i64));
        x += 0.2;
    }
    out
}

#[test]
fn real_math_functions_compile_and_match_ieee754() {
    let text = run(ALL_FUNCTIONS);
    assert_compiled(&text);
    let got: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("R|") || l.starts_with("I|"))
        .map(str::to_string)
        .collect();
    assert_eq!(got, expected_all());
}

/// The shape of #1326: the call is on a branch that never runs, and it used
/// to keep the whole block off compiled code all the same.
const DEAD_BRANCH: &str = r#"
`timescale 1ns/1ns
module top;
  real x = 0.0;
  int  n = 0;
  always #(1) begin
    x = x * 0.999 + 1.0e-3;
    n = n + 1;
    if (x < -1.0) x = $sqrt(-x) + $tanh(x) + $rtoi(x);
    if (n == 1000) begin
      $display("D|%0d|%016h", n, $realtobits(x));
      $finish;
    end
  end
endmodule
"#;

/// A math call inside a function the process calls: the function is pure in
/// its arguments, so it inlines and the process compiles (#1326).
const CALLED_FUNCTION: &str = r#"
`timescale 1ns/1ns
module top;
  real x = 0.25;
  int  n = 0;
  function automatic real f(input real a);
    return $tanh(a) + $sqrt(a);
  endfunction
  always #(1) begin
    x = f(x) * 0.5;
    n = n + 1;
    if (n == 50) begin
      $display("F|%016h", $realtobits(x));
      $finish;
    end
  end
endmodule
"#;

#[test]
fn math_call_in_a_called_function_keeps_the_process_compiled() {
    let text = run(CALLED_FUNCTION);
    assert_compiled(&text);
    assert!(
        !text.lines().any(|l| l.contains("Expr_Call_impure")),
        "the math function made its caller impure:\n{text}"
    );
    let mut x = 0.25_f64;
    for _ in 0..50 {
        x = (x.tanh() + x.sqrt()) * 0.5;
    }
    let want = format!("F|{:016x}", x.to_bits());
    assert!(text.lines().any(|l| l == want), "want {want}:\n{text}");
}

#[test]
fn math_call_on_an_untaken_branch_keeps_the_process_compiled() {
    let text = run(DEAD_BRANCH);
    assert_compiled(&text);
    let mut x = 0.0_f64;
    for _ in 0..1000 {
        x = x * 0.999 + 1.0e-3;
    }
    let want = format!("D|1000|{:016x}", x.to_bits());
    assert!(text.lines().any(|l| l == want), "want {want}:\n{text}");
}
