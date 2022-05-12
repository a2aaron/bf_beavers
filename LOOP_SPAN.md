# Detecting infinite loops in Brainfuck

In order to determine if a given BF program is a busy-beaver champion, we need
to be able to decide if the program ever halts. While this is impossible in the 
general case, we can do pretty well for lots of cases.

## Approach 1: Symbolic Execution
This approach is probably the better one, compared to what I'm actually doing.
It is however, harder to implement and I haven't bothered do it yet.

## A Loop Span Example
Consider the following program `+[>>+++]`. Take a look at the execution of
the program below.
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
Clearly, we will keep looping execution. We know this happens because, 
at the end of the loop, we are always looking at a cell with value `03`, which
causes another execution of the loop to occur, shifts us over and writes `03` to 
a cell, which we look at a cell with value `03` at the end, and so on.

We can note that each execution of the loop only touches a small chunk
of memory. We call the memory that was touched a "LoopSpan":
```brainfuck
LoopSpan of loop #1
[01 00 00 00 00 00 00]
 ^^ 
[01 00 03 00 00 00 00]
 ----- ^^

LoopSpan of loop #2
[01 00 03 00 00 00 00]
       ^^
[01 00 03 00 03 00 00]
       -- -- ^^

LoopSpan of loop #3
[01 00 03 00 03 00 00]
             ^^
[01 00 03 00 03 00 03]
             -- -- ^^
```

Here, we see two types of loop spans. Loop #1 has a LoopSpan consisting of the 
memory `[01 00 03]`, and a memory pointer that starts at `01` and ends at `03`.

However, Loops #2 and #3 have a different LoopSpan. They have a LoopSpan that
consists of the memory `[03 00 03]`, and a memory pointer that starts at the 
first `03` and ends at the second `03`. 
