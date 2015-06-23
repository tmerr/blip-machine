#![feature(slice_patterns)]
#![feature(iter_arith)]
extern crate rand;
use std::io::Read;
use std::io::Write;
use std::collections::HashMap;
use rand::{Rng, SeedableRng, StdRng};
use rand::distributions::{IndependentSample, Range};

macro_rules! println_stderr(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

static PROGRAM_NAME: &'static str = "blip-machine";

enum CompileError {
    Syntax(usize),
    Lbl(usize),
    Prob(usize),
    Num(usize),
}

/// Takes in a textual program and converts it to a list of instructions.
/// This can later be interpreted by the magic virtual sound machine.
fn compile(text: &str) -> Result<Vec<Instruction>, Vec<CompileError>> {
    let mut instructions = vec![];
    let mut errors = vec![];

    let mut lbls = HashMap::new();
    let mut ctr = 0;
    for line in text.lines() {
        let splt: Vec<&str> = line.split(" ").collect();
        if let ["lbl", ident] = splt.as_ref() {
            lbls.insert(ident, ctr);
        } else {
            // line == 1: blank line, ignored
            if splt.len() > 1 {
                ctr += 1
            }
        }
    }

    for (i, line) in text.lines().enumerate() {
        let splt: Vec<&str> = line.split(" ").collect();
        match splt.as_ref() {
            ["lbl", _] => {},
            ["sin", freq, dur] => {
                let freqf: f64 = match freq.parse() {
                    Ok(num) => num,
                    _ => { errors.push(CompileError::Num(i)); continue; },
                };
                let durf: f64 = match dur.parse() {
                    Ok(num) => num,
                    _ => { errors.push(CompileError::Num(i)); continue; },
                };
                instructions.push(Sin(freqf, durf));
            },
            ["pjump", lbl, prob] => {
                let linenum = match lbls.get(&lbl) {
                    Some(num) => *num,
                    None => { errors.push(CompileError::Lbl(i)); continue; },
                };
                let probf: f64 = match prob.parse() {
                    Ok(num) => { 
                        if 0.0 <= num && num <= 1.0 {
                            num
                        } else {
                            errors.push(CompileError::Prob(i));
                            continue;
                        }
                    }
                    Err(_) => { errors.push(CompileError::Num(i)); continue; }
                };
                instructions.push(PJump(probf, linenum as usize));
            },
            ["pfork", lbl, prob] => {
                let linenum = match lbls.get(&lbl) {
                    Some(num) => *num,
                    None => { errors.push(CompileError::Lbl(i)); continue; },
                };
                let probf: f64 = match prob.parse() {
                    Ok(num) => { 
                        if 0.0 <= num && num <= 1.0 {
                            num
                        } else {
                            errors.push(CompileError::Prob(i));
                            continue;
                        }
                    }
                    Err(_) => { errors.push(CompileError::Num(i)); continue; }
                };
                instructions.push(PFork(probf, linenum as usize));
            },
            _ => {
                // blank line
                if splt.len() == 1 && splt[0] == "" {
                    continue;
                }
                // unknown line
                errors.push(CompileError::Syntax(i));
                continue;
            },
        }
    }
    instructions.push(Terminate);

    return if errors.len() == 0 {
        Ok(instructions)
    } else {
        Err(errors)
    };
}

fn print_errors(lst: &Vec<CompileError>) {
    for err in lst.iter() {
        match *err {
            CompileError::Syntax(line) => {
                println_stderr!("{}:{} error: bad syntax", PROGRAM_NAME, line);
            },
            CompileError::Lbl(line) => {
                println_stderr!("{}:{} error: unknown label", PROGRAM_NAME, line);
            },
            CompileError::Prob(line) => {
                println_stderr!("{}:{} error: probabilities must be between 0 and 1", PROGRAM_NAME, line);
            },
            CompileError::Num(line) => {
                println_stderr!("{}:{} error: expected a number", PROGRAM_NAME, line);
            },
        }
    }
    println_stderr!("\nerror: aborting due to {} previous errors.", lst.len());
}

pub use Instruction::*;
enum Instruction {
    Sin(f64, f64),
    PJump(f64, usize),
    PFork(f64, usize),
    Terminate,
}

#[derive(Clone)]
struct ThreadState {
    sin_progress: i64,
    pc: usize
}

static SAMPLE_RATE: f64 = 8000.0;

/// sample a sine wave in range -1 to 1
fn sine_wave(freq: f64, step: i64) -> f64 {
    (2.0*std::f64::consts::PI*(step as f64)*freq/SAMPLE_RATE).sin()
}

/// have all threads interpret until they're lined up at a sin instruction
fn interpret_to_sin<R: Rng>(threads: &Vec<ThreadState>, instructions: &[Instruction], rng: &mut R) -> Vec<ThreadState> {
    fn bernoulli_trial<R: Rng>(p: f64, rng: &mut R) -> bool {
        let sample = Range::new(0_f64, 1_f64).ind_sample(rng);
        p > sample
    }

    // Interpret, branching out like a tree, spawning nodes at forks, and killing
    // nodes when the program counter reaches the terminate instruction.
    fn recurse<R: Rng>(thread: ThreadState, instructions: &[Instruction], rng: &mut R) -> Vec<ThreadState> {
        match instructions[thread.pc] {
            Sin(_, _) => {
                return vec![thread];
            },
            PJump(p, line) => {
                return if bernoulli_trial(p, rng) {
                    recurse(ThreadState { sin_progress: 0, pc: line }, instructions, rng)
                } else {
                    recurse(ThreadState { sin_progress: 0, pc: thread.pc + 1 }, instructions, rng)
                }
            },
            PFork(p, line) => {
                return if bernoulli_trial(p, rng) { 
                    [recurse(ThreadState { sin_progress: 0, pc: line }, instructions, rng),
                    recurse(ThreadState { sin_progress: 0, pc: thread.pc + 1 }, instructions, rng)
                    ].concat()
                } else {
                    recurse(ThreadState { sin_progress: 0, pc: thread.pc + 1 }, instructions, rng)
                }
            },
            Terminate => {
                return vec![];
            }
        }
    }

    let mut result = vec![];
    for thread in threads.iter() {
        result.extend(recurse(thread.clone(), &instructions, rng));
    }
    result
}

/// play the sound for this time step
/// pre: all threads are at a sin instruction
fn interpret_sin(threads: &Vec<ThreadState>, instructions: &[Instruction]) -> Vec<ThreadState> {
    let mut new_threads = vec![];
    let mut current_samples = vec![];

    for thread in threads {
        if let Sin(freq, duration) = instructions[thread.pc] {
            if (thread.sin_progress as f64) < duration*SAMPLE_RATE {
                current_samples.push(sine_wave(freq, thread.sin_progress));
                new_threads.push(ThreadState { sin_progress: thread.sin_progress + 1, pc: thread.pc } );
            } else {
                new_threads.push(ThreadState { sin_progress: 0, pc: thread.pc + 1 }); 
            }
        }
        else {
            panic!("interpret_sin precondition not met");
        }
    }

    let avg = current_samples.iter().sum::<f64>() / (current_samples.len() as f64);
    let sample: [u8; 1] = [(127.5_f64*(1_f64 + avg)) as u8];
    std::io::stdout().write_all(&sample).unwrap();    

    new_threads
}

fn build_rand() -> StdRng {
    let seed: &[usize] = &[0];
    let rng: StdRng = SeedableRng::from_seed(seed);
    rng
}

/// Interprets the list of instructions and produces sound. This "sound" is really
/// an 8-bit 8000Hz PCM stream sent through stdout. It can be piped into something
/// like aplay.
fn interpret(instructions: &[Instruction]) {
    let mut threads = vec![ThreadState{ sin_progress:0, pc: 0 }];
    let mut rand = build_rand();
    while threads.len() != 0 {
        threads = interpret_to_sin(&threads, &instructions, &mut rand);
        threads = interpret_sin(&threads, &instructions);
    }
}

fn main() {
    let mut inp = std::io::stdin();
    let mut text = String::new();
    inp.read_to_string(&mut text).unwrap();
    match compile(&text) {
        Ok(instructions) => {
            interpret(&instructions);
        },
        Err(errors) => {
            print_errors(&errors);
        }
    }
}
