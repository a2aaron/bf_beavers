# Busy Beaver Brainfuck
Brainfuck is an esoteric programming language consisting of an extremely small number of commands. An example of a Brainfuck program is shown below. When run, it outputs "Hello World!".

```brainfuck
++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.
```
(Source: https://en.wikipedia.org/wiki/Brainfuck#Hello_World!)

It is designed to be difficult to program due to its extremely simple design. Many interesting questions can be asked about this programming language. One of them is the Busy Beaver problem, which is as follows:

What is the _longest running_ Brainfuck program consisting of exactly `N` characters, for each `N`?

Note that the Brainfuck program must terminate--it cannot run forever (otherwise obviously the longest running Brainfuck program is just any program which loops forever).

Note: Some variations of the Busy Beaver problem exist--for example, we could ask about the Brainfuck program that alters the most memory cells, or prints the most output. However, I am choosing to focus on run time, because I think this is the most pure translation of the original Busy Beaver problem for Turing Machines.

## Brainfuck Semantics

The particular semantics of Brainfuck differ between implementations subtly. I have chosen to use the following semantics:

- A Brainfuck program consists of six instructions: `+`, `-`, `>`, `<`, `[`, and `]`. All other characters are ignored.
- Every `[` must have a corresponding `]`. Programs which do not are invalid.
- There exists a **memory tape**, also called the memory, which is an array of cells that extends infinitely to the right. The cells can be indexed starting from zero. The zeroth-cell is called the _first cell_
- There exists a **memory pointer** that can point to a single cell at any given time. The cell currently pointed by the memory pointer is called the _current cell_
- There exists a **program pointer**, which points to the current instruction to be executed.
- Cells may take on values from `00` to `FF`.
- At the beginning of execution, every cell is initialized to zero and the memory pointer points to the first cell.
- At each step of execution, the instruction at the program pointer is executed (according to the descriptions below). Then, the program pointer is incremented by one. If the program pointer is no longer pointing to any instructions (and hence has reached the end of the program), then the program halts.
- `+` - If the current cell's value is not `FF`, then increment the value of the cell. Otherwise, the cell's value instead wraps around to `00`.
- `-` - If the current cell's value is not `00`, then decrement the value of the cell. Otherwise, the cell's value instead wraps around to `FF`.
- `>` - Move the memory pointer to the cell to the right. That is to say, the memory pointer now points to the cell whose index is one higher than was pointed to before.
- `<` - If the memory pointer is not pointing to the first cell, then move the memory pointer to the cell to the left. If the memory pointer is already pointing to the first cell, then this instruction does nothing.
- `[` - If the current cell's value is zero, then jump to the matching `]`. Otherwise, do nothing. (Note that the program pointer is still incremented, so this effectively means that `[` either jumps one past the matching `]`, or is a no-op. For example, in the program `[++--++]<>>`, the order of execution of instructions is `[>>>`)
- `]` - If the current cell's value is non-zero, then jump to the matching `[`. Otherwise, do nothing. (Note that the program pointer is still incremented, so this effectively means that `[` either jumps one past the matching `[` or is a no-op. For example, in the program `---[+]>>>`, the order of execution of instructions is `---[+]+]+]>>`).

This is similar to most other Brainfuck implementations. A list of notable differences is listed below:

- There is no `.` or `,` instruction. This means that there is no user input or output. The `,` instruction is absent because we need programs to execute deterministically, and user input would make this impossible. The `.` instruction is absent because this would effectly be a no-op, which is boring and uninteresting. (at best, `.` would simply serve to pad out the number of BF programs which exist without meaningfully changing the semantics). This also allows me to entirely sidestep the issues of newline and EOF handling.
- Cell values are in the range `00` to `FF` and are explicitly defined to wrap. This matches lots of other implementations and allows for common constructs like `[-]` to mean "zero out a cell"
- The tape is infinite towards the right. Having a finite tape seems boring and doesn't match most implementatinons anyways.
- Attempting to move off the tape towards the left does nothing. This avoids needing to implement an "error" state and introduces an interesting asymmetry--moving to the right is always possible, but moving towards the left eventually is not possible. Hence, programs blindly move towards the left eventually experience different behavior from programs which blindly move towards the right (which means it isn't possible to simply swap `>` and `<` and still have the same program).
- `[` and `]` do not jump directly onto the matching brace, but instead one past the brace. This is to avoid useless additional current cell checks, and to more clealy allow the transformation of a Brainfuck loop into a `while` loop construct.

## The Halting Problem

In order to determine if a given BF program is a busy-beaver champion, we need to be able to decide if the program ever halts. While this is impossible in the general case, we can do pretty well for lots of cases.

## Approach 1: Full-State Cycle Detection

### Program State
The state required to store a currently executing Brainfuck program is quite small. Given a program, the required state consists of:
- The program pointer
- The memory pointer
- The memory

The program pointer and memory pointer are very cheap and just consists of a `usize` each. The memory itself is also very cheap. Since all cells are initially zero at the start of execution, we can store only the cells which are accessed. There are a bunch of ways we could do this, but the simplest is to just store the "loaded" part of memory in a vector and grow the vector as nessecary as we move to the right into new cells. For small programs, this is typically extremely small. (We could try to improve this scheme, for example by only allocating memory if we write into a cell that we then later will read from, but this method is sufficent for now).

### The Method
Given this simplicity, it seems like we have a simple method of determining if a program has entered a cycle: Store an "execution history" of prior program states and check if the current state ever matches a prior state. If so, then this cycle must continue (since program execution is deterministic and we have recorded everything that could possibly affect the program state, nothing could ever break us out of the loop).

Now, saving state for every instruction seems somewhat wasteful. We only can get into an infinite loop whenever a we have a loop-construct. Hence, we can instead associate each loop with an execution history. Each time we reach the start of the loop (which happens either when initially going into the loop via `[` or by jumping backwards via `]`), we check the current state against the execution history list. If the current state appears in the list, then we know an infinite loop has occured. Otherwise, we add the current state to the execution history list and continue.

In pseudo-Rust, the loop-detection algorithm might look like this:

```rust
fn loop_detector(program: SomeBrainfuckProgram) {
    while program has not halted {
        run program for one step
        // In other words, if the program is at the instruction just past a 
        // start_loop instruction (aka: [).
        if program just entered the start of a loop {
            // The current state is a snapshot of the program's memory and 
            // memory pointer. Note that we don't need the program pointer, 
            // since that is implicitly part of the loop_id below.
            let current_state = program.memory and program.memory_pointer
           
            // This could be something simple, like the index of the start loop
            // or end loop instruction associated with this loop. In the actual 
            // implementation, we use the start loop index.
            let loop_id = the loop that we are currently in
            
            // If this is the first time we have entered this loop, then the 
            // execution_history is empty. Otherwise, it should contain the set
            // of all the previously seen program states.
            let execution_history = execution_histories[loop_id]
            // If the current state is in the execution history, then that means
            // we have reached a prior state and that this program must be 
            // looping.
            if current_state is in execution_history {
                signal infinite loop detected
            }
            // We then add the current state to the execution history for the 
            // next program iteration to use 
            add current_state to execution_history
        }
    }

    // If we reach here, then our program halted, which we can also signal
    signal program halted
}
```

### An Example

Here is an example. Consider the following program: `+>+>+[<]`. We have the
following execution:
```brainfuck
Start of Program
[00 00 00] +>+>+[<]
 ^^        ^

Start of loop #1 ([ entered)
[01 01 01] +>+>+[<]
       ^^       ^
[01 01 01] +>+>+[<]
    ^^           ^
[01 01 01] +>+>+[<]
    ^^            ^

Start of loop #2 (] taken)
[01 01 01] +>+>+[<]
 ^^              ^
[01 01 01] +>+>+[<]
 ^^               ^

Start of loop #3 ([ taken)
[01 01 01] +>+>+[<]
 ^^              ^
[01 01 01] +>+>+[<]
 ^^               ^

Start of loop #4 (] taken)

!! Loop Detected (Loop #4 is identical to Loop #3) !!
```

Note that we would detect the loop upon taking the backedge associated with `]`. This would mean that a loop be detected on executing the `]`, and the full history of instructions before the loop is detected is `+>+>+>[<]<]<]`

### Problems
This type of cycle detection does not catch all types of loops. For example, it cannot catch `+[>+]`. This is because the loop consists of an infinite chain of new states. The memory pointer constantly moves right, causing each execution of the loop to view itself as being in a different state. However, it's clear that the loop can never actually exit the loop, and that all of these states are "the same" in some sense. 

## Approach 2: Loop Spans
Consider the behavior of the program `+[>+]` after a long time. The cells near the beginning of the tape are no longer important to the program's execution.  Hence, we can visualize execution as follows:

```brainfuck
... 01 01 01 00 00 00 ... +[>+]
          ^^                ^

... 01 01 01 00 00 00 ... +[>+]
             ^^              ^

... 01 01 01 01 00 00 ... +[>+]
             ^^               ^

... 01 01 01 01 00 00 ... +[>+]
             ^^             ^
```

But notice this visualization suggests that the last and first states are identical! On the `>` instruction we have a memory which looks like "some ones, then followed by an infinite sea of zeros, with the memory pointer at the cell just before the sea of zeros". It seems that, for the purposes of loop detection, we ought to be able to only consider a subset of the entire memory snapshot when comparing loop executions. 

In particular, we can note that a given loop's execution can only touch a certain region of memory. Anything outside that region must be unable to affect execution. If we could determine that one particular loop's execution gives rise to another loop's execution, we could use that to determine infinite loops!

But how to do this?

## A Loop Span Example
Consider the following program `+[>>+++]`. Take a look at the execution of the program below.
```brainfuck
Start of execution
[00 00 00 00 00 00 00] +[>>+++]
 ^^                    ^

Start of loop #1
[01 00 00 00 00 00 00] +[>>+++]
 ^^                     ^
 
End of loop #1
[01 00 03 00 00 00 00] +[>>+++]
       ^^                     ^

Start of loop #2
[01 00 03 00 00 00 00] +[>>+++]
       ^^               ^

End of loop #2
[01 00 03 00 03 00 00] +[>>+++]
             ^^               ^

Start of loop #3
[01 00 03 00 03 00 00] +[>>+++]
             ^^         ^
End of loop #3
[01 00 03 00 03 00 03] +[>>+++]
                   ^^         ^
```
Clearly, we will keep looping execution. We know this happens because, at the end of the loop, we are always looking at a cell with value `03`, which causes another execution of the loop to occur, shifts us over and writes `03` to a cell, which we look at a cell with value `03` at the end, and so on.

### Touched Regions

We can note that each execution of the loop only touches a small chunk of memory. We call the memory that was touched a **touched region**:
```brainfuck
Touched region of loop #1
[01 00 00 __ __ __ __] (Start)
 ^^ 
[01 00 03 __ __ __ __] (End)
 ----- ^^

Touched region of loop #2
[__ __ 03 00 00 __ __] (Start)
       ^^
[__ __ 03 00 03 __ __] (End)
       -- -- ^^

Touched region of loop #3
[__ __ __ __ 03 00 00] (Start)
             ^^
[__ __ __ __ 03 00 03] (End)
             -- -- ^^
```
Note that a touched region's values consist of the value they had at the _start_ of the loop, and include all the cells which were ever accessed over the _entire duration of the loop_. (We show the end state of the tape for reference, but the end state of the tape is not considered part of the touched region. Hence, Loop #1 has a region consisting of the memory `[01 00 00]`, not `[01 00 03]`.

The reason for using the starting values of memory, rather than the ending values, is that the ending state may not be unique. It is possible for two touched regions to have the same ending region but have different starting regions. For example, `++[[-]++]` has touched regions for `[[-]++]` with different starting values but end with the same values:

```brainfuck
Initial execution of [[-]++]
[02] ++[[-]++]
 ^^    ^
[02] ++[[-]++]
 ^^         ^

Subsequent executions of [[-]++]
[02] ++[[-]++]
 ^^    ^
[02] ++[[-]++]
 ^^         ^
```

Since execution must stay within the touched region for the duration of the loop, we are guarenteed that the starting memory values will always become the ending memory values within the touched region--any value outside the touched region cannot possibly affect the execution of the loop-code (if such a cell could, the cell must have be read or written to at some point during the loop, but then it would need to be in the touched region!). 

### Displacement

Note that, just because two touched regions match does not imply that their _long term behaviors_ are the same (that is, it doesn't imply that the executions will both either halt or not halt). First, we need their **displacements** to match as well. The displacement is the number of cells that the memory pointer moves between the starting and ending cells.

In the program `+[>>+++]` that we traced above, the displacements are as follows: For Loop #1, the displacement is +2, because it has a memory pointer that starts at cell 0 and ends at cell 2. For Loop #2 and #3, the displacements are also +2. 

Note that the displacement can be zero or negative. For example, the loops in `+[>><<]` have a displacement of zero, and the displacements of the loops in `+>+>+>+>+>+[<]` are negative one.

### Extension

Finally, we also must consider the **extension**. The extension consists of the region of memory to the left or to the right of the touched region. If the displacement is positive, then the extension is all of the cells to the right of the touched region (including the Infinite Sea of Zeros). If the displacement is negative, then the extension is all of the cells to the left of the touched region, down to the first cell. If the displacement is zero, then there is no extension.

For an example of why the touched regions are not enough, consider the following program: `+>>>>-<<<<[>+]`

```brainfuck
Start of execution
[00 00 00 00 00 00 00] +>>>>-<<<<[>+]
 ^^                    ^

Loop #1 Start
[01 00 00 00 FF 00 00] +>>>>-<<<<[>+]
 ^^                              ^

Loop #1 End/Loop #2 Start
[01 01 00 00 FF 00 00] +>>>>-<<<<[>+]
    ^^                           ^

Loop #2 End/Loop #3 Start
[01 01 01 00 FF 00 00] +>>>>-<<<<[>+]
       ^^                        ^

Loop #3 End/Loop #4 Start
[01 01 01 01 FF 00 00] +>>>>-<<<<[>+]
          ^^                     ^

Loop #4 End
[01 01 01 01 00 00 00] +>>>>-<<<<[>+]
             ^^                     ^
Halt.
```

Due to the existence of the `>>>>-<<<<` setting one of the cells to `FF`, we end up breaking out of the `[>+]` loop. Note that for each of the loops, the  touched region looks like:
```brainfuck
[00 00]
 ^^
[01 00]
 -- ^^
```

Hence, we have a program whose loops have the same touched regions and the same displacements, but doesn't loop! This is because the positive displacement means that future executions of the loop will be affected by cells to the right. Hence we must consider not just the cells which actually affected execution this loop (which is the touched region), but also the cells which could possibly _ever_ affect future executions of this loop. Hence, we must include the entire right portion of the tape. When doing this, we can see that the loop spans are not identical. For example, here are a few of the loop spans encountered during the loop:

```brainfuck
Loop Span #1
[01 00 00 00 FF 00 00]
 -- ^^                

Loop Span #2
[__ 01 00 00 FF 00 00]
    -- ^^ 

Loop Span #3
[__ __ 01 00 FF 00 00]
       -- ^^ 

Loop Span #4
[__ __ __ 01 FF 00 00] 
          -- ^^              
```

Comparing the program's touched region along with the extension reveals that we actually have different long-term behaviors! 

### Putting it All Together
We now have all the components of a Loop Span. A Loop Span for some execution of a loop consists of:
- The touched region
- The displacement
- The extension  
Where the values of the touched region and extension are taken from the values at the start of the loop. From this we have the following loop-detection algorithm:
```rust
fn loop_detector(program: SomeBrainfuckProgram) {
    while program has not halted {
        let instr = next program instruction to be executed
        if instr == StartLoop and loop is taken {
            // In this case, we have just entered a loop for the first time
            // Like before, we need some unique ID for each loop in the program
            let loop_id = the loop we have just entered
            // A span recorder will track the overall behavior that the program 
            // does over the course of the given loop. It will contain a 
            // snapshot of the memory + memory pointer as it exists right now, 
            // as well as tracking the maximum horizontal displacements and 
            // final displacement. We add a new span recorder associated with 
            // the current loop we are in. The recorder will record until we 
            // hit the corresponding EndLoop instruction 
            add new_span_recorder to active_loop_spans[loop_id] 
        } else if instr == EndLoop and loop is taken {
            let loop_id = the loop we have just entered

            // swap out the old span recording for a new one
            let finished_span_recording = active_loop_spans[loop_id]
            add new_span_recorder to active_loop_spans[loop_id] 

            // This is a pretty easy operation--we just have to determine the 
            // overall displacement to get the extension, use the maximum 
            // horizontal displacements to get the touched region.
            let loop_span = finished_span_recording.compute_loop_span()

            // Now, check if the loop_span already exists in the loop span 
            // history of this loop. If it does, then we know we've hit an 
            // infinite loop.  
            if loop_span in past_loop_spans[loop_id] {
                signal infinite loop
            }

            // Finally, add this loop_span to the loop span history
            add loop_span to past_loop_spans[loop_id]
        }

        // Finally, run the actual instruction and continue
        execute instruction
    }

    // If we reach here, then our program halted, which we can also signal
    return program halted
}

```
