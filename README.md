# turing-lang
A Rust library for parsing .tur and .mtur files, which can construct Turing machines.

## Getting started
### Running a .tur file
.tur files can be run using the command `compute` by `path/to/binary compute path/to/turfile input` where `input` is a string which has each symbol separated by a space. For example:
```
> ./turing-machine compute add_one.tur '0 0 1 1'

                                        v
_____________________________________0011_______________________________________
                                       v
_____________________________________0010_______________________________________
                                      v
_____________________________________0000_______________________________________
                                      v
_____________________________________0100_______________________________________
    ACCEPTED
```

### How a .tur file is constructed
Let's look at `add_one.tur` and go though it step by step:
```
# everything after '#' on a line is a comment
# adds one to tape contents if parsed as a binary number
# will overflow, as in input tape '1111' will give output '0000'
states a
syms 0 1
initstate a
finalstates HALT
table
a 0 HALT 1 N
a 1 a 0 L
a _ HALT . R
```
The first three lines are comments, and are not considered when building the machine. 

* The `states` command on line 4 defines the state `a`. For a state to be usable it has to first be defined with a `states` command. `states` can take in any amount of state names separated with spaces.
* The `syms` command defines the set of allowed symbols on the tape, in this case `0` and `1`. As with `states`, a symbol needs to be defined with a `syms` command to be usable.
* The `initstate` command defines the initial state of the machine. This can only be used once in a file. 
* The `finalstates` command defines a set of states which are valid `HALT` states. Note that the state `HALT` is used even though it is not defined. `HALT` is a special state which is always defined for every machine.
* The `table` command signals the start of the machines' transition table and should always be put last in the file. Each line describes a state/action pair of sorts, which takes five tokens (`s`, `r`, `ns`, `w`, `dir`). For each computation step, the machine will go though the table in order and do the first action which matches. If the machine is in state `s` and reads the symbol `r` it will switch to the state `ns`, write the symbol `w` to the tape and move either left (`L`), right (`R`) or stay (`N`). Note that the symbol `_` is used on the last line, even though it is not defined anywhere. This is a special *blank* symbol, and is (like `HALT`) always defined for every machine.

For the table there are some special characters. `*` is a wildcard operator, which will match for any state or symbol. `.` can be seen as a noop which means "don't switch state/write nothing". Or equivalently, "switch to the same state you were in/write what you read". You can also define multiple states or symbols on one line by separating them via a comma but *no* space. For example `a,b,d`.

### Chaining machines
Anytime a *machine* is used in a command you send in its path to the .tur file. Machines can be chained via the command `chain`. For example, the command
```
turing chain [MACHINE1] [MACHINE2] [OUTPUT]
```
will take the machines at `MACHINE1` and `MACHINE2`, construct a new Turing machine which operates as the two machines would do in a sequence where the output from the first machine gets fed as input into the other. The resulting .tur file is then written to `OUTPUT`.

### Branching machines
Machines can also be branching via the command `branch`. It should be used as 
```
turing branch [ENTRY_MACHINE] [BRANCH_SYMS] [BRANCH_MACHINES] [OUTPUT]
```
The argument `ENTRY_MACHINE` is the machine which is used as the entry point for the entire program. When `ENTRY_MACHINE` halts the symbol under the pointer is read. This symbol determines which machine should be chained next. The arguments `BRANCH_SYMS` and `BRANCH_MACHINES` are two strings, where `BRANCH_SYMS` are all relevant symbols separated by a space, and `BRANCH_MACHINES` are each corresponding machine, also each separated by a space. The resulting .tur file is then written to `OUTPUT`.

That was a mouthful! Let's go though an example usage. Assume we have the files `nand.tur`, `and.tur` and `stop.tur` which performs the operations NAND and AND. The machine `stop.tur` simply halts. We also have the machine `find_op.tur` which will find the left-most operator symbol and then halt on top of it. We can now create a branching machine via the command
```
turing branch next_op.tur '& D _' 'and.tur nand.tur stop.tur' logic.tur
```
This creates a machine `logic.tur`. Let's try running it!
```
> turing compute logic.tur '0 & 1'

                                        v
______________________________________0&1_______________________________________
                                       v
______________________________________0&1_______________________________________
                                      v
______________________________________0&1_______________________________________
                                     v
______________________________________0&1_______________________________________
                                      v
______________________________________0&1_______________________________________
                                       v
______________________________________0&1_______________________________________
                                       v
______________________________________0&1_______________________________________
                                        v
______________________________________0&1_______________________________________
                                       v
______________________________________0&________________________________________
                                      v
______________________________________0_________________________________________
                                      v
______________________________________1_________________________________________
                                      v
______________________________________0_________________________________________
    ACCEPTED

> turing compute logic.tur '0 D 1'

                                        v
______________________________________0D1_______________________________________
                                       v
______________________________________0D1_______________________________________
                                      v
______________________________________0D1_______________________________________
                                     v
______________________________________0D1_______________________________________
                                      v
______________________________________0D1_______________________________________
                                       v
______________________________________0D1_______________________________________
                                       v
______________________________________0D1_______________________________________
                                        v
______________________________________0D1_______________________________________
                                       v
______________________________________0D________________________________________
                                      v
______________________________________0_________________________________________
                                      v
______________________________________1_________________________________________
    ACCEPTED
```
We now have a machine which can handle different operators!

## The operation symbols
| Symbol | Operation |
--- | ---
`D` | nand
`!` | not 
`&` | and
`\|` | or 
`:` | generic operation