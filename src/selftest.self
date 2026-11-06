"A self-test of the language and the starting world, written in Self.
 Run it with:  selfTest run   (or World > Run the self-test)."

lobby _AddSlots: (|
  selfTest = (|
    {} = 'Checks that the Self engine and its world work. Send me run.'.
    parent* = traits clonable.
    passed <- 0.
    failed <- 0.
    failures <- nil.

    { 'Category: checking'
      check: name Gives: expected By: block = ( | got |
        got: [ block value ] onError: [| :m | 'error: ', m ].
        got = expected
          ifTrue: [ passed: passed + 1 ]
          False: [ failed: failed + 1.
                   failures add: name, ': got ', got printString, ', expected ', expected printString ].
        self ) }.

    { 'Category: helpers'
      firstEven: v = ( v do: [| :x | x even ifTrue: [ ^ x ] ]. nil ).
      countDown: n = ( n = 0 ifTrue: 0 False: [ countDown: n - 1 ] ) }.

    { 'Category: running'
      run = (
        passed: 0. failed: 0. failures: list copyRemoveAll.
        'Self World self-test' printLine.

        "numbers"
        check: 'addition' Gives: 7 By: [ 3 + 4 ].
        check: 'binary messages go left to right' Gives: 14 By: [ 3 + 4 * 2 ].
        check: 'unary before binary' Gives: 8 By: [ 2 + 3 factorial ].
        check: 'keyword after binary' Gives: 27 By: [ 3 + 4 factorial max: 20 ].
        check: 'integer division' Gives: 3 By: [ 7 / 2 ].
        check: 'float division' Gives: 3.5 By: [ 7 / 2.0 ].
        check: 'modulo' Gives: 2 By: [ -7 % 3 ].
        check: 'radix literal' Gives: 255 By: [ 16rFF ].
        check: 'big integers' Gives: '265252859812191058636308480000000' By: [ 30 factorial printString ].
        check: 'big integer arithmetic' Gives: 9900 By: [ 100 factorial / 98 factorial ].
        check: 'square root' Gives: 5.0 By: [ 25 sqrt ].
        check: 'primes' Gives: true By: [ 97 isPrime ].

        "booleans and nil"
        check: 'ifTrue:False:' Gives: 'yes' By: [ (3 > 2) ifTrue: [ 'yes' ] False: [ 'no' ] ].
        check: 'and / or' Gives: true By: [ (true && [ false not ]) || [ false ] ].
        check: 'nil' Gives: true By: [ nil isNil ].

        "strings"
        check: 'concatenation' Gives: 'hello world' By: [ 'hello', ' world' ].
        check: 'reverse and uppercase' Gives: 'CBA' By: [ 'abc' reverse asUppercase ].
        check: 'characters are strings' Gives: 'b' By: [ 'abc' at: 1 ].
        check: 'words' Gives: 3 By: [ 'the quick fox' words size ].

        "objects, slots and prototypes"
        check: 'object literal with a method' Gives: 7 By: [ (| x <- 3. y = 4. sum = ( x + y ) |) sum ].
        check: 'assignment slots' Gives: 9 By: [ | o | o: (| x <- 1 |). o x: 9. o x ].
        check: 'copies are separate' Gives: 70 By: [ | a | a: bankAccount copy. a deposit: 100. a withdraw: 30. a balance ].
        check: 'the prototype is untouched' Gives: 0 By: [ bankAccount balance ].
        check: 'points' Gives: '4@5' By: [ ((3 @ 4) + (1 @ 1)) printString ].
        check: 'point equality' Gives: true By: [ (3 @ 4) = (3 @ 4) ].
        check: 'adding slots' Gives: 2 By: [ | o | o: (| |) _Clone. o _AddSlots: (| n <- 1. bump = ( n: n + 1 ) |). o bump. o n ].
        check: 'reflection' Gives: '(\'parent\', \'x\', \'y\')' By: [ (reflect: 3 @ 4) names printString ].

        "inheritance"
        check: 'inherited method' Gives: 1 By: [ | o | o: (| parent* = list copyRemoveAll |). o add: 5. o size ].
        check: 'resend' Gives: 'a!' By: [ | p. c | p: (| parent* = traits clonable. say = 'a' |).
                                         c: (| parent* = p. say = ( resend.say, '!' ) |).
                                         c say ].
        check: 'dynamic inheritance' Gives: 'ab' By: [ | a. b. c. r |
                                         a: (| parent* = traits clonable. v = 'a' |).
                                         b: (| parent* = traits clonable. v = 'b' |).
                                         c: (| p* <- nil |).
                                         c p: a. r: c v. c p: b. r, c v ].

        "blocks"
        check: 'block with arguments' Gives: 42 By: [ [| :a. :b | a * b ] value: 6 With: 7 ].
        check: 'closures' Gives: 15 By: [ | adder | adder: [| :n | [| :x | x + n ] ]. (adder value: 10) value: 5 ].
        check: 'non-local return' Gives: 8 By: [ firstEven: (3 & 7 & 8 & 9) asVector ].
        check: 'loops' Gives: 5050 By: [ | r | r: 0. 1 to: 100 Do: [| :i | r: r + i ]. r ].
        check: 'whileTrue:' Gives: 10 By: [ | i | i: 0. [ i < 10 ] whileTrue: [ i: i + 1 ]. i ].
        check: 'recursion' Gives: 0 By: [ countDown: 150 ].

        "collections"
        check: 'vectors' Gives: '(1, 2, 3)' By: [ (1 & 2 & 3) asVector printString ].
        check: 'sorting' Gives: '(1, 4, 5)' By: [ (5 & 1 & 4) asVector sort printString ].
        check: 'collect and sum' Gives: 385 By: [ (1 to: 10 Collect: [| :i | i * i ]) sum ].
        check: 'select' Gives: 3 By: [ ((1 to: 6 Collect: [| :i | i ]) select: [| :x | x even ]) size ].
        check: 'inject:Into:' Gives: 24 By: [ (2 & 3 & 4) asVector inject: 1 Into: [| :a. :e | a * e ] ].
        check: 'lists grow' Gives: 100 By: [ | l | l: list copyRemoveAll. 1 to: 100 Do: [| :i | l add: i ]. l size ].
        check: 'dictionaries' Gives: 2 By: [ | d | d: dictionary copyRemoveAll. d at: 'a' Put: 1. d at: 'b' Put: 2. d at: 'b' ].

        "errors"
        check: 'message not understood' Gives: 'error: 3 does not understand \'foo\'' By: [ 3 foo ].
        check: 'primitive failure handler' Gives: 'divisionByZeroError' By: [ 1 _IntDiv: 0 IfFail: [| :e | e ] ].

        "morphs"
        check: 'morphs are objects' Gives: 'a circle morph' By: [ circleMorph copy printString ].
        check: 'adding a submorph' Gives: true By: [ | p. c | p: morph copy. c: labelMorph copy. p addMorph: c. c owner == p ].
        check: 'copies are deep' Gives: false By: [ | p | p: morph copy. p addMorph: labelMorph copy. p copy morphs first == p morphs first ].
        check: 'buttons run their action' Gives: 1 By: [ | n. b | n: 0. b: buttonMorph copyLabel: 'x' Action: [ n: n + 1 ]. b click. n ].
        check: 'deleting a morph' Gives: 0 By: [ | p. c | p: morph copy. c: morph copy. p addMorph: c. c delete. p morphs size ].

        "original Self file format"
        check: 'bootstrap paths' Gives: true By: [ (bootstrap stub -> 'globals' -> 'true' -> ()) == true ].
        check: 'eval' Gives: 7 By: [ '3 + 4' eval ].

        report ) }.

    { 'Category: printing'
      report = (
        failures do: [| :f | ('  FAILED  ', f) printLine ].
        (passed printString, ' passed, ', failed printString, ' failed') printLine.
        failed = 0
          ifTrue: [ 'All ', passed printString, ' checks passed' ]
          False: [ failed printString, ' of ', (passed + failed) printString, ' checks failed (see the transcript)' ] ) }
  |)
|).
