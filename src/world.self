"The starting world for Self World, written in Self.

 Like the original Self system, almost all behaviour lives in objects:
 the lobby inherits from globals and defaultBehavior, traits objects hold
 shared behaviour, and prototypes are copied to make new objects. Only the
 primitives (messages starting with an underscore) are built into the VM."

defaultBehavior _AddSlots: (|
  {} = 'The default behaviour for the lobby and every object that inherits from it.'.
  { 'Category: comparing'
    == x = ( _Eq: x ).
    !== x = ( (_Eq: x) not ).
    = x = ( _Eq: x ).
    != x = ( (self = x) not ).
    hash = ( _IdentityHash ).
    identityHash = ( _IdentityHash ) }.
  { 'Category: printing'
    printString = ( _ObjectName ).
    print = ( printString _StringPrint. self ).
    printLine = ( printString _StringPrintLine. self ).
    asString = ( printString ).
    name = ( _ObjectName ) }.
  { 'Category: testing'
    isNil = false.
    notNil = true.
    ifNil: b = ( self ).
    ifNotNil: b = ( b value: self ).
    ifNil: nb IfNotNil: b = ( b value: self ).
    isNumber = false.
    isString = false.
    isVector = false.
    isBlock = false.
    isBoolean = false }.
  { 'Category: evaluating'
    value = ( self ).
    yourself = ( self ) }.
  { 'Category: collecting'
    & x = ( (list copyRemoveAll add: self) add: x ) }.
  { 'Category: errors'
    error: msg = ( _Error: msg ).
    halt = ( _Error: 'halt' ) }.
  { 'Category: reflection'
    inspect = ( _Inspect ).
    reflect: x = ( mirror copy reflectee: x ).
    slotNames = ( _SlotNames ).
    kindName = ( _KindName ).
    perform: sel = ( _Perform: sel ).
    perform: sel With: a = ( _Perform: sel With: a ).
    perform: sel With: a With: b = ( _Perform: sel With: a With: b ) }
|).

traits _AddSlots: (|
  clonable = (|
    {} = 'Objects that may be copied: most prototypes inherit from me.'.
    parent* = lobby.
    copy = ( _Clone ).
    clone = ( _Clone ) |).
  oddball = (|
    {} = 'Unique objects such as nil, true and numbers: copying returns the object itself.'.
    parent* = lobby.
    copy = ( self ).
    clone = ( self ) |)
|).

traits _AddSlots: (|
  boolean = (| parent* = traits oddball. isBoolean = true |).
  number = (| parent* = traits oddball |).
  collection = (| parent* = traits clonable |)
|).

traits _AddSlots: (|
  indexable = (| parent* = traits collection |)
|).

"---------------------------------------------------------------- booleans and nil"

true _AddSlots: (|
  parent* = traits boolean.
  { 'Category: control'
    ifTrue: b = ( b value ).
    ifFalse: b = ( nil ).
    ifTrue: a False: b = ( a value ).
    ifFalse: a True: b = ( b value ) }.
  { 'Category: logic'
    && b = ( b value ).
    || b = ( true ).
    and: b = ( b value ).
    or: b = ( true ).
    ^^ b = ( b value not ).
    not = false }.
  printString = 'true'.
  asInteger = 1
|).

false _AddSlots: (|
  parent* = traits boolean.
  { 'Category: control'
    ifTrue: b = ( nil ).
    ifFalse: b = ( b value ).
    ifTrue: a False: b = ( b value ).
    ifFalse: a True: b = ( a value ) }.
  { 'Category: logic'
    && b = ( false ).
    || b = ( b value ).
    and: b = ( false ).
    or: b = ( b value ).
    ^^ b = ( b value ).
    not = true }.
  printString = 'false'.
  asInteger = 0
|).

nil _AddSlots: (|
  {} = 'The object that slots hold before anything else is put in them.'.
  parent* = traits oddball.
  isNil = true.
  notNil = false.
  ifNil: b = ( b value ).
  ifNotNil: b = ( nil ).
  ifNil: b IfNotNil: nb = ( b value ).
  printString = 'nil'
|).

"---------------------------------------------------------------- numbers"

traits number _AddSlots: (|
  { 'Category: testing'
    isNumber = true.
    isZero = ( self = 0 ).
    sign = ( self > 0 ifTrue: 1 False: [ self < 0 ifTrue: -1 False: 0 ] ) }.
  { 'Category: comparing'
    max: x = ( self > x ifTrue: self False: x ).
    min: x = ( self < x ifTrue: self False: x ).
    between: a And: b = ( (self >= a) && [ self <= b ] ).
    != x = ( (self = x) not ) }.
  { 'Category: arithmetic'
    negated = ( 0 - self ).
    abs = ( self < 0 ifTrue: [ self negated ] False: self ).
    squared = ( self * self ).
    sqrt = ( asFloat sqrt ).
    pi = 3.141592653589793 }.
  { 'Category: geometry'
    @ y = ( (point copy x: self) y: y ) }
|).

traits integer _AddSlots: (|
  {} = 'Shared behaviour of all integers, small and big. Mixing in a float gives a float.'.
  parent* = traits number.
  { 'Category: arithmetic'
    + a = ( _IntAdd: a IfFail: [ asFloat + a ] ).
    - a = ( _IntSub: a IfFail: [ asFloat - a ] ).
    * a = ( _IntMul: a IfFail: [ asFloat * a ] ).
    "Integers never overflow: they grow into big integers, as in the original Self."
    / a = ( _IntDiv: a IfFail: [| :e | e = 'divisionByZeroError'
                                   ifTrue: [ error: 'division by zero' ]
                                   False: [ asFloat / a ] ] ).
    % a = ( _IntMod: a IfFail: [ error: 'division by zero' ] ).
    rem: a = ( _IntRem: a IfFail: [ error: 'division by zero' ] ).
    succ = ( self + 1 ).
    pred = ( self - 1 ).
    raisedTo: n = ( | r <- 1 | n timesRepeat: [ r: r * self ]. r ).
    factorial = ( | r <- 1 | self < 0 ifTrue: [ ^ error: 'factorial of a negative number' ].
      2 to: self Do: [| :i | r: r * i ]. r ).
    fibonacci = ( self < 2 ifTrue: self False: [ (self - 1) fibonacci + (self - 2) fibonacci ] ).
    gcd: n = ( n = 0 ifTrue: [ self abs ] False: [ n gcd: self % n ] ).
    lcm: n = ( (self * n) abs / (gcd: n) ) }.
  { 'Category: comparing'
    < a = ( _IntLT: a IfFail: [ asFloat < a ] ).
    <= a = ( _IntLE: a IfFail: [ asFloat <= a ] ).
    > a = ( _IntGT: a IfFail: [ asFloat > a ] ).
    >= a = ( _IntGE: a IfFail: [ asFloat >= a ] ).
    = a = ( _IntEQ: a IfFail: [ a isNumber ifTrue: [ asFloat = a ] False: false ] ).
    hash = ( self ) }.
  { 'Category: testing'
    even = ( (self % 2) = 0 ).
    odd = ( even not ).
    isPrime = ( | d <- 2 |
      self < 2 ifTrue: [ ^ false ].
      [ (d * d) <= self ] whileTrue: [
        (self % d) = 0 ifTrue: [ ^ false ].
        d: d + 1 ].
      true ) }.
  { 'Category: bits'
    bitAnd: a = ( _IntAnd: a ).
    bitOr: a = ( _IntOr: a ).
    bitXor: a = ( _IntXor: a ).
    << n = ( _IntArithmeticShiftLeft: n ).
    >> n = ( _IntArithmeticShiftRight: n ) }.
  { 'Category: iterating'
    to: e Do: b = ( | i | i: self. [ i <= e ] whileTrue: [ b value: i. i: i + 1 ]. self ).
    to: e By: s Do: b = ( | i |
      i: self.
      s > 0 ifTrue: [ [ i <= e ] whileTrue: [ b value: i. i: i + s ] ]
            False: [ [ i >= e ] whileTrue: [ b value: i. i: i + s ] ].
      self ).
    upTo: e Do: b = ( | i | i: self. [ i < e ] whileTrue: [ b value: i. i: i + 1 ]. self ).
    downTo: e Do: b = ( | i | i: self. [ i >= e ] whileTrue: [ b value: i. i: i - 1 ]. self ).
    timesRepeat: b = ( | i <- 0 | [ i < self ] whileTrue: [ b value. i: i + 1 ]. self ).
    to: e Collect: b = ( | v | v: vector copySize: ((e - self) + 1) max: 0.
      to: e Do: [| :i | v at: i - self Put: (b value: i) ].
      v ) }.
  { 'Category: converting'
    asFloat = ( _IntAsFloat ).
    asInteger = ( self ).
    truncate = ( self ).
    rounded = ( self ).
    asCharacter = ( _IntAsCharacter ).
    random = ( _Random: self ) }.
  { 'Category: printing'
    printString = ( _IntPrintString ).
    printStringBase: b = ( _IntPrintStringBase: b ).
    hexPrintString = ( _IntPrintStringBase: 16 ) }
|).

traits float _AddSlots: (|
  {} = 'Shared behaviour of all floating point numbers.'.
  parent* = traits number.
  { 'Category: arithmetic'
    + a = ( _FloatAdd: a ).
    - a = ( _FloatSub: a ).
    * a = ( _FloatMul: a ).
    / a = ( _FloatDiv: a IfFail: [ error: 'division by zero' ] ).
    % a = ( _FloatMod: a ).
    raisedTo: n = ( _FloatPow: n ).
    sqrt = ( _FloatSqrt ).
    sin = ( _FloatSin ).
    cos = ( _FloatCos ).
    tan = ( _FloatTan ).
    arcTan = ( _FloatArcTan ).
    ln = ( _FloatLn ).
    exp = ( _FloatExp ) }.
  { 'Category: comparing'
    < a = ( _FloatLT: a ).
    <= a = ( _FloatLE: a ).
    > a = ( _FloatGT: a ).
    >= a = ( _FloatGE: a ).
    = a = ( _FloatEQ: a IfFail: false ) }.
  { 'Category: converting'
    asFloat = ( self ).
    asInteger = ( _FloatAsInt ).
    truncate = ( _FloatTruncate ).
    rounded = ( _FloatRound ).
    floor = ( _FloatFloor ).
    ceil = ( _FloatCeil ) }.
  printString = ( _FloatPrintString )
|).

"---------------------------------------------------------------- collections"

traits collection _AddSlots: (|
  {} = 'Behaviour shared by every collection. Children supply size and do:.'.
  { 'Category: testing'
    isEmpty = ( size = 0 ).
    notEmpty = ( size > 0 ).
    includes: x = ( do: [| :e | e = x ifTrue: [ ^ true ] ]. false ).
    indexOf: x = ( do: [| :e. :i | e = x ifTrue: [ ^ i ] ]. -1 ) }.
  { 'Category: enumerating'
    inject: acc Into: b = ( | r | r: acc. do: [| :e | r: (b value: r With: e) ]. r ).
    detect: b = ( detect: b IfNone: nil ).
    detect: b IfNone: nb = ( do: [| :e | (b value: e) ifTrue: [ ^ e ] ]. nb value ).
    count: b = ( | n <- 0 | do: [| :e | (b value: e) ifTrue: [ n: n + 1 ] ]. n ).
    anySatisfy: b = ( do: [| :e | (b value: e) ifTrue: [ ^ true ] ]. false ).
    allSatisfy: b = ( do: [| :e | (b value: e) ifFalse: [ ^ false ] ]. true ).
    select: b = ( | l | l: list copyRemoveAll. do: [| :e | (b value: e) ifTrue: [ l add: e ] ]. l asVector ).
    reject: b = ( select: [| :e | (b value: e) not ] ).
    collect: b = ( | l | l: list copyRemoveAll. do: [| :e | l add: (b value: e) ]. l asVector ) }.
  { 'Category: math'
    sum = ( inject: 0 Into: [| :a. :e | a + e ] ).
    max = ( | m | do: [| :e | (m isNil || [ e > m ]) ifTrue: [ m: e ] ]. m ).
    min = ( | m | do: [| :e | (m isNil || [ e < m ]) ifTrue: [ m: e ] ]. m ).
    average = ( sum asFloat / size ) }.
  { 'Category: converting'
    asVector = ( | v. i <- 0 | v: vector copySize: size. do: [| :e | v at: i Put: e. i: i + 1 ]. v ).
    asList = ( | l | l: list copyRemoveAll. do: [| :e | l add: e ]. l ) }
|).

traits indexable _AddSlots: (|
  {} = 'Collections whose elements are numbered from 0, such as vectors and strings.'.
  { 'Category: enumerating'
    do: b = ( 0 upTo: size Do: [| :i | b value: (at: i) With: i ]. self ).
    reverseDo: b = ( (size - 1) downTo: 0 Do: [| :i | b value: (at: i) With: i ]. self ).
    collect: b = ( | r | r: vector copySize: size.
      0 upTo: size Do: [| :i | r at: i Put: (b value: (at: i) With: i) ].
      r ).
    with: other Do: b = ( 0 upTo: size Do: [| :i | b value: (at: i) With: (other at: i) ]. self ) }.
  { 'Category: accessing'
    first = ( at: 0 ).
    last = ( at: size - 1 ) }
|).

traits _AddSlots: (|
  vector = (| parent* = traits indexable |)
|).

globals _AddSlots: (|
  vector = (| parent* = traits vector |) _MakeVector
|).

traits vector _AddSlots: (|
  {} = 'Fixed-size arrays of any objects. Make new ones with: vector copySize: 3'.
  { 'Category: accessing'
    size = ( _Size ).
    at: i = ( _At: i IfFail: [ error: 'index ', i printString, ' is outside 0 to ', (size - 1) printString ] ).
    at: i Put: x = ( _At: i Put: x IfFail: [ error: 'index ', i printString, ' is outside 0 to ', (size - 1) printString ] ) }.
  { 'Category: copying'
    copySize: n = ( _Clone: n Filler: nil ).
    copySize: n FillingWith: x = ( _Clone: n Filler: x ).
    copyAddLast: x = ( (copySize: size + 1) at: size Put: x ).
    , v = ( | r | r: (copySize: size + v size).
      0 upTo: v size Do: [| :i | r at: size + i Put: (v at: i) ].
      r ).
    reverse = ( | r | r: (copySize: size).
      0 upTo: size Do: [| :i | r at: (size - 1) - i Put: (at: i) ].
      r ).
    "sortBy: (a stable merge sort) is with the original protocol further down."
    sort = ( sortBy: [| :a. :b | a < b ] ) }.
  isVector = true.
  printString = ( | s <- '' |
    do: [| :e. :i | i > 0 ifTrue: [ s: s, ', ' ]. s: s, e printString ].
    '(', s, ')' )
|).

traits _AddSlots: (|
  list = (| parent* = traits indexable |)
|).

globals _AddSlots: (|
  list = (| parent* = traits list. items <- vector copySize: 4. count <- 0 |)
|).

traits list _AddSlots: (|
  {} = 'A collection that grows as you add to it: list copyRemoveAll add: 3'.
  { 'Category: adding'
    add: x = (
      count = items size ifTrue: [ items: items copySize: (items size * 2) max: 4 ].
      items at: count Put: x.
      count: count + 1.
      self ).
    addLast: x = ( add: x ).
    addAll: c = ( c do: [| :e | add: e ]. self ).
    & x = ( add: x ) }.
  { 'Category: removing'
    removeLast = ( | x | isEmpty ifTrue: [ ^ error: 'the list is empty' ].
      count: count - 1. x: items at: count. items at: count Put: nil. x ).
    removeFirst = ( | x | isEmpty ifTrue: [ ^ error: 'the list is empty' ].
      x: items at: 0.
      1 upTo: count Do: [| :i | items at: i - 1 Put: (items at: i) ].
      count: count - 1.
      x ).
    removeAll = ( count: 0. items: vector copySize: 4. self ) }.
  { 'Category: accessing'
    size = ( count ).
    at: i = ( ((i < 0) || [ i >= count ]) ifTrue: [ ^ error: 'index ', i printString, ' is outside the list' ].
      items at: i ).
    at: i Put: x = ( ((i < 0) || [ i >= count ]) ifTrue: [ ^ error: 'index ', i printString, ' is outside the list' ].
      items at: i Put: x. self ).
    first = ( at: 0 ).
    last = ( at: count - 1 ) }.
  { 'Category: enumerating'
    do: b = ( 0 upTo: count Do: [| :i | b value: (items at: i) With: i ]. self ) }.
  { 'Category: copying'
    copy = ( resend.copy items: items copy ).
    copyRemoveAll = ( copy removeAll ).
    asVector = ( items copySize: count ) }.
  printString = ( 'list', asVector printString )
|).

traits _AddSlots: (|
  dictionary = (| parent* = traits collection |)
|).

globals _AddSlots: (|
  dictionary = (| parent* = traits dictionary. keys <- list copyRemoveAll. values <- list copyRemoveAll |)
|).

traits dictionary _AddSlots: (|
  {} = 'Keys mapped to values: dictionary copyRemoveAll at: 1 Put: 2'.
  { 'Category: accessing'
    at: k Put: v = ( | i | i: keys indexOf: k.
      i < 0 ifTrue: [ keys add: k. values add: v ] False: [ values at: i Put: v ].
      self ).
    at: k = ( at: k IfAbsent: [ error: 'key not found: ', k printString ] ).
    at: k IfAbsent: b = ( | i | i: keys indexOf: k. i < 0 ifTrue: [ b value ] False: [ values at: i ] ).
    includesKey: k = ( (keys indexOf: k) >= 0 ).
    removeKey: k = ( | i. nk. nv | i: keys indexOf: k.
      i < 0 ifTrue: [ ^ self ].
      nk: list copyRemoveAll. nv: list copyRemoveAll.
      keys do: [| :e. :j | j = i ifFalse: [ nk add: e. nv add: (values at: j) ] ].
      keys: nk. values: nv. self ).
    size = ( keys size ) }.
  { 'Category: enumerating'
    do: b = ( values do: [| :v. :i | b value: v With: (keys at: i) ]. self ).
    keysDo: b = ( keys do: b. self ) }.
  { 'Category: copying'
    copy = ( (resend.copy keys: keys copy) values: values copy ).
    copyRemoveAll = ( (resend.copy keys: list copyRemoveAll) values: list copyRemoveAll ) }.
  printString = ( | s <- '' |
    keys do: [| :k. :i | i > 0 ifTrue: [ s: s, ', ' ]. s: s, k printString, ' -> ', (values at: i) printString ].
    'dictionary(', s, ')' )
|).

"---------------------------------------------------------------- strings"

traits string _AddSlots: (|
  {} = 'Immutable strings. Characters are strings of length one; indices start at 0.'.
  parent* = traits indexable.
  { 'Category: accessing'
    size = ( _StringSize ).
    at: i = ( _StringAt: i IfFail: [ error: 'index ', i printString, ' is outside the string' ] ).
    byteAt: i = ( _StringByteAt: i ).
    asByte = ( _StringByteAt: 0 ) }.
  { 'Category: comparing'
    = s = ( _StringEQ: s ).
    < s = ( _StringLT: s ).
    > s = ( s < self ).
    hash = ( _StringHash ) }.
  { 'Category: copying'
    , s = ( _StringConcat: s asString ).
    copyFrom: a UpTo: b = ( _StringCopyFrom: a UpTo: b IfFail: [ error: 'bad range ', a printString, ' to ', b printString ] ).
    copyFrom: a = ( copyFrom: a UpTo: size ).
    reverse = ( _StringReverse ).
    asUppercase = ( _StringAsUppercase ).
    asLowercase = ( _StringAsLowercase ).
    trim = ( _StringTrim ).
    capitalize = ( isEmpty ifTrue: self False: [ (copyFrom: 0 UpTo: 1) asUppercase, (copyFrom: 1) ] ) }.
  { 'Category: searching'
    includesSubstring: s = ( (_StringIndexOf: s) >= 0 ).
    indexOfSubstring: s = ( _StringIndexOf: s ).
    split: sep = ( _StringSplit: sep ).
    words = ( _StringSplit: ' ' ).
    lines = ( _StringSplit: '\n' ) }.
  { 'Category: converting'
    asString = ( self ).
    asInteger = ( _StringAsIntegerIfFail: [ error: 'not an integer: ', self printString ] ).
    asFloat = ( _StringAsFloatIfFail: [ error: 'not a number: ', self printString ] ).
    asNumber = ( _StringAsIntegerIfFail: [ asFloat ] ) }.
  { 'Category: printing'
    printString = ( _StringPrintString ).
    print = ( _StringPrint ).
    printLine = ( _StringPrintLine ) }.
  { 'Category: evaluating'
    eval = ( _StringEval ) }.
  isString = true
|).

"---------------------------------------------------------------- blocks"

traits block _AddSlots: (|
  {} = 'Blocks are closures: [| :x | x * 2 ] value: 21'.
  parent* = traits oddball.
  { 'Category: looping'
    whileTrue: b = ( _WhileTrue: b ).
    whileFalse: b = ( _WhileFalse: b ).
    whileTrue = ( _WhileTrue: [] ).
    whileFalse = ( _WhileFalse: [] ).
    "As in the original: run me, then stop once b gives true (a do-until loop)."
    untilTrue: b = ( [ value. b value ifTrue: [ ^ nil ] ] loop ).
    untilFalse: b = ( untilTrue: [ b value not ] ).
    loop = ( _Loop ) }.
  { 'Category: evaluating'
    numArgs = ( _BlockNumArgs ).
    valueWithArguments: v = ( _ValueWithArguments: v ).
    onError: handler = ( _OnError: handler ).
    msecToRun = ( | t | t: _TimeMsec. value. _TimeMsec - t ) }.
  isBlock = true.
  printString = 'a block'
|).

traits method _AddSlots: (|
  parent* = traits oddball.
  printString = 'a method'
|).

"---------------------------------------------------------------- points, mirrors, utilities"

traits _AddSlots: (|
  point = (| parent* = traits clonable |).
  mirror = (| parent* = traits clonable |)
|).

globals _AddSlots: (|
  point = (| parent* = traits point. x <- 0. y <- 0 |).
  mirror = (| parent* = traits mirror. reflectee <- nil |)
|).

traits point _AddSlots: (|
  {} = 'Cartesian points. Make one with 3 @ 4'.
  { 'Category: arithmetic'
    "Like the original, the argument may be a point or a number."
    + p = ( | q | q: p asPoint. copyX: x + q x Y: y + q y ).
    - p = ( | q | q: p asPoint. copyX: x - q x Y: y - q y ).
    * p = ( | q | q: p asPoint. copyX: x * q x Y: y * q y ).
    / p = ( | q | q: p asPoint. copyX: x / q x Y: y / q y ).
    r = ( (x squared + y squared) sqrt ).
    dist: p = ( ((x - p x) squared + (y - p y) squared) sqrt ) }.
  { 'Category: comparing'
    = p = ( (p _Eq: self) || [ (x = p x) && [ y = p y ] ] ) }.
  printString = ( x printString, '@', y printString )
|).

traits mirror _AddSlots: (|
  {} = 'A mirror lets you look at an object\'s slots as data.'.
  names = ( reflectee _SlotNames ).
  at: n = ( reflectee _SlotValue: n ).
  size = ( names size ).
  kind = ( reflectee _KindName ).
  addSlots: o = ( reflectee _AddSlots: o. self ).
  removeSlot: n = ( reflectee _RemoveSlot: n. self ).
  printString = ( 'a mirror on ', reflectee printString )
|).

globals _AddSlots: (|
  transcript = (|
    {} = 'The transcript window. transcript show: 42; printLine also writes here.'.
    parent* = traits oddball.
    show: s = ( s asString _StringPrint. self ).
    cr = ( '' _StringPrintLine. self ).
    clear = ( _TranscriptClear ) |).
  time = (|
    {} = 'Milliseconds since Self World started.'.
    parent* = traits oddball.
    msec = ( _TimeMsec ).
    printString = ( 'time ', _TimeMsec printString, ' ms' ) |).
  random = (|
    {} = 'random integer: 6 gives 0 to 5.'.
    parent* = traits oddball.
    integer: n = ( _Random: n ).
    between: a And: b = ( a + (_Random: (b - a) + 1) ) |).
  memory = (|
    parent* = traits oddball.
    objectCount = ( _ObjectCount ) |)
|).

lobby _AddSlots: (|
  { 'Category: examples'
  bankAccount = (|
    {} = 'The classic Self example: copy me, then deposit: and withdraw:'.
    parent* = traits clonable.
    balance <- 0.
    deposit: x = ( balance: balance + x ).
    withdraw: x = ( balance: (balance - x) max: 0 ).
    printString = ( 'an account with ', balance printString ) |).
  stack = (|
    {} = 'A stack built by inheriting from list and renaming its messages.'.
    parent* = list copyRemoveAll.
    push: x = ( add: x ).
    pop = ( removeLast ).
    top = ( last ) |) }
|).

"---------------------------------------------------------------- protocol from the original core world

 Several methods in this section are adapted from the original Self system
 (objects/core in https://github.com/russellallen/self), Copyright 2009-2026
 the Self AUTHORS, based on Self 4.3, Copyright 1993-2006 Sun Microsystems, Inc.
 and Stanford University. They are used under the Self licence; its copyright
 notices and disclaimer are reproduced in the file NOTICE.

 The messages below come from the original Self core files (objects/core in
 the Self distribution: defaultBehavior, number, integer, float, collection,
 indexable, list, dictionary, setAndDictionary, string, block, point and
 rectangle). They keep the original names and meanings, so code written for
 the original Self runs here too. Parts that need the original virtual
 machine (byte vectors, files, processes, profiling) are left out."

defaultBehavior _AddSlots: (|
  { 'Category: evaluating'
    "Any object can stand in for a block that ignores its arguments."
    value: a = ( self ).
    value: a With: b = ( self ).
    value: a With: b With: c = ( self ).
    value: a With: b With: c With: d = ( self ).
    value: a With: b With: c With: d With: e = ( self ).
    value: a With: b With: c With: d With: e With: f = ( self ).
    value: a With: b With: c With: d With: e With: f With: g = ( self ) }.
  { 'Category: testing'
    isNotNil = ( notNil ).
    ifNotNil: b IfNil: nb = ( b value: self ).
    isRectangle = false.
    isList = false.
    isMorph = false }.
  { 'Category: errors'
    halt: msg = ( _Error: 'halt: ', msg asString ).
    warning: msg = ( ('warning: ', msg asString) printLine. self ).
    quietError: msg = ( _Error: msg ) }.
  { 'Category: printing'
    printStringSize: n = ( | s | s: printString.
      s size <= n ifTrue: [ s ] False: [ (s copyFrom: 0 UpTo: (n - 3) max: 0), '...' ] ).
    "A Self expression that rebuilds the object: an object literal, or the object's name."
    storeString = ( storeStringIfFail: [ error: 'no store string for ', printString ] ).
    storeStringIfFail: b = ( _FileOut ).
    namePrintString = ( _ObjectName ) }.
  { 'Category: reflection'
    asMirror = ( reflect: self ) }
|).

nil _AddSlots: (|
  ifNotNil: b IfNil: nb = ( nb value ).
  storeStringIfFail: b = ( printString )
|).

traits boolean _AddSlots: (|
  { 'Category: comparing'
    "false comes before true, as in the original."
    < b = ( asInteger < b asInteger ).
    <= b = ( asInteger <= b asInteger ).
    > b = ( asInteger > b asInteger ).
    >= b = ( asInteger >= b asInteger ).
    max: b = ( self < b ifTrue: [ b ] False: [ self ] ).
    min: b = ( self < b ifTrue: [ self ] False: [ b ] ).
    compare: b IfLess: l Equal: e Greater: g = ( asInteger compare: b asInteger IfLess: l Equal: e Greater: g ) }.
  storeStringIfFail: b = ( printString )
|).

"---- numbers"

traits number _AddSlots: (|
  { 'Category: comparing'
    compare: x IfLess: l Equal: e Greater: g = (
      self < x ifTrue: [ l value ] False: [ self = x ifTrue: [ e value ] False: [ g value ] ] ) }.
  { 'Category: arithmetic'
    absoluteValue = ( abs ).
    negate = ( negated ).
    square = ( squared ).
    squareRoot = ( sqrt ).
    inverse = ( 1.0 / asFloat ).
    succ = ( self + 1 ).
    pred = ( self - 1 ).
    successor = ( succ ).
    predecessor = ( pred ).
    double = ( self + self ).
    half = ( self / 2 ).
    mean: x = ( (self + x) /~ 2 ) }.
  { 'Category: children'
    zero = 0.
    one = 1.
    two = 2 }.
  { 'Category: functions'
    asBoolean = ( self != 0 ).
    asPoint = ( self @ self ).
    power: n = ( raisedTo: n ).
    "Whole number of times the receiver fits in a by multiplying: 2 log: 1024 gives 10."
    log: a = ( | e <- 1. m <- 0 |
      a <= 0 ifTrue: [ ^ error: 'log: negative or zero argument' ].
      self <= 1 ifTrue: [ ^ error: 'log: receiver <= 1' ].
      [ e <= a ] whileTrue: [ e: e * self. m: m + 1 ].
      m - 1 ).
    logBase: b = ( asFloat ln / b asFloat ln ).
    ln = ( asFloat ln ).
    exp = ( asFloat exp ).
    sin = ( asFloat sin ).
    cos = ( asFloat cos ).
    tan = ( asFloat tan ).
    arcTan = ( asFloat arcTan ).
    "The angle of the point x @ self, like atan2."
    arcTan: x = ( | yy. xx |
      yy: asFloat. xx: x asFloat.
      xx > 0 ifTrue: [ ^ (yy / xx) arcTan ].
      xx < 0 ifTrue: [ ^ yy >= 0 ifTrue: [ (yy / xx) arcTan + pi ] False: [ (yy / xx) arcTan - pi ] ].
      yy > 0 ifTrue: [ ^ pi / 2 ].
      yy < 0 ifTrue: [ ^ pi negated / 2 ].
      0.0 ).
    interpolate: p From: x = ( (asFloat * (1.0 - p asFloat)) + (x asFloat * p asFloat) ).
    sumTo: e = ( | s <- 0 | to: e Do: [| :i | s: s + i ]. s ).
    sumFrom: a To: e = ( | s | s: self. a to: e Do: [| :i | s: s + i ]. s ).
    roundUpTo: n = ( (self /+ n) * n ).
    roundDownTo: n = ( (self /- n) * n ) }.
  { 'Category: printing'
    printStringPrecision: n = ( | m. r. s |
      m: 10 raisedTo: n.
      r: (abs asFloat * m) rounded.
      s: (r / m) printString.
      n > 0 ifTrue: [ s: s, '.', ((r % m) printStringPadWith: '0' ToSize: n) ].
      self < 0 ifTrue: [ '-', s ] False: [ s ] ).
    storeStringIfFail: b = ( printString ) }
|).

traits integer _AddSlots: (|
  { 'Category: arithmetic'
    "/- rounds down, /+ rounds up, /~ rounds to nearest, /= is exact (a float if it must be)."
    /- a = ( self / a ).
    /+ a = ( ((0 - self) / a) negated ).
    /~ a = ( (self + (a / 2)) / a ).
    /= a = ( (self % a) = 0 ifTrue: [ self / a ] False: [ asFloat / a ] ) }.
  { 'Category: bits'
    && a = ( bitAnd: a ).
    || a = ( bitOr: a ).
    ^^ a = ( bitXor: a ).
    complement = ( -1 - self ).
    numberOfOnes = ( | n. c <- 0 |
      self < 0 ifTrue: [ ^ error: 'numberOfOnes of a negative number' ].
      n: self. [ n > 0 ] whileTrue: [ c: c + (n bitAnd: 1). n: n >> 1 ]. c ) }.
  { 'Category: iterating'
    do: b = ( 0 upTo: self Do: b ).
    reverseDo: b = ( (self - 1) downTo: 0 Do: b ).
    upTo: e By: s Do: b = ( | i | i: self. [ i < e ] whileTrue: [ b value: i. i: i + s ]. self ).
    downTo: e By: s Do: b = ( | i | i: self. [ i >= e ] whileTrue: [ b value: i. i: i - s ]. self ).
    to: e ByPositive: s Do: b = ( to: e By: s Do: b ).
    to: e ByNegative: s Do: b = ( to: e By: s Do: b ) }.
  { 'Category: converting'
    floor = ( self ).
    ceil = ( self ).
    round = ( self ).
    asCharacterIfFail: b = ( _IntAsCharacterIfFail: b ) }.
  { 'Category: number theory'
    factors = ( | l. n. d <- 2 |
      l: list copyRemoveAll. n: self abs.
      [ (d * d) <= n ] whileTrue: [
        (n % d) = 0 ifTrue: [ l add: d. n: n / d ] False: [ d: d + 1 ] ].
      n > 1 ifTrue: [ l add: n ].
      l asVector ).
    nextPrime = ( | n | n: self + 1. [ n isPrime ] whileFalse: [ n: n + 1 ]. n ).
    power: e Mod: m = ( | r. b. k |
      r: 1. b: self % m. k: e.
      [ k > 0 ] whileTrue: [
        k odd ifTrue: [ r: (r * b) % m ].
        b: (b * b) % m.
        k: k / 2 ].
      r ).
    "The inverse of the receiver modulo m: (3 inverseMod: 7) * 3 % 7 = 1."
    inverseMod: m = ( | t <- 0. nt <- 1. r. nr. q. tmp |
      r: m. nr: self % m.
      [ nr != 0 ] whileTrue: [
        q: r / nr.
        tmp: t - (q * nt). t: nt. nt: tmp.
        tmp: r - (q * nr). r: nr. nr: tmp ].
      r > 1 ifTrue: [ ^ error: self printString, ' has no inverse modulo ', m printString ].
      t % m ).
    asSmallInteger = ( self ) }.
  { 'Category: printing'
    decimalPrintString = ( printString ).
    binaryPrintString = ( printStringBase: 2 ).
    octalPrintString = ( printStringBase: 8 ).
    printStringBase: b PadWith: c ToSize: n = ( | s |
      s: printStringBase: b.
      [ s size < n ] whileTrue: [ s: c, s ].
      s ).
    printStringPadWith: c ToSize: n = ( printStringBase: 10 PadWith: c ToSize: n ).
    printStringPadWith0ToSize: n = ( printStringPadWith: '0' ToSize: n ) }
|).

traits float _AddSlots: (|
  { 'Category: arithmetic'
    /- a = ( (self / a) floor ).
    /+ a = ( (self / a) ceil ).
    /~ a = ( self / a ).
    /= a = ( self / a ) }.
  { 'Category: children'
    zero = 0.0.
    one = 1.0.
    two = 2.0 }.
  { 'Category: converting'
    round = ( rounded ) }
|).

"---- collections"

traits collection _AddSlots: (|
  { 'Category: testing'
    nonEmpty = ( notEmpty ).
    noneSatisfy: b = ( (anySatisfy: b) not ).
    includesAll: c = ( c do: [| :e | (includes: e) ifFalse: [ ^ false ] ]. true ) }.
  { 'Category: accessing'
    first = ( firstIfAbsent: [ error: 'empty collection' ] ).
    firstIfAbsent: b = ( do: [| :e | ^ e ]. b value ).
    lastIfAbsent: b = ( | l. found <- false | do: [| :e | l: e. found: true ]. found ifTrue: [ l ] False: [ b value ] ).
    ifNone: nb = ( isEmpty ifTrue: [ nb value ] False: [ self ] ).
    ifNone: nb IfOne: ob IfMany: mb = ( | sole |
      doFirst: [| :e | sole: e ] MiddleLast: [ ^ mb value ] IfEmpty: [ ^ nb value ].
      ob value: sole ).
    soleElement = ( ifNone: [ error: 'the collection is empty' ]
                    IfOne: [| :e | e ]
                    IfMany: [ error: 'the collection has more than one element' ] ) }.
  { 'Category: iterating with ends'
    "f runs on the first element, m on the middle ones, lst on the last; e if there are none."
    doFirst: f Middle: m Last: lst IfEmpty: e = ( | i <- 0. pk. pv |
      do: [| :v. :k |
        i = 0 ifTrue: [ i: 1 ] False: [
          (i = 1 ifTrue: [ i: 2. f ] False: [ m ]) value: pv With: pk ].
        pv: v. pk: k ].
      i = 0 ifTrue: [ ^ e value ].
      (i = 1 ifTrue: [ f ] False: [ lst ]) value: pv With: pk.
      self ).
    doFirst: f Middle: m Last: lst = ( doFirst: f Middle: m Last: lst IfEmpty: [] ).
    doFirst: f MiddleLast: ml IfEmpty: e = ( doFirst: f Middle: ml Last: ml IfEmpty: e ).
    doFirst: f MiddleLast: ml = ( doFirst: f Middle: ml Last: ml IfEmpty: [] ).
    doFirstLast: fl Middle: m IfEmpty: e = ( doFirst: fl Middle: m Last: fl IfEmpty: e ).
    doFirstLast: fl Middle: m = ( doFirst: fl Middle: m Last: fl IfEmpty: [] ).
    doFirstMiddle: fm Last: lst IfEmpty: e = ( doFirst: fm Middle: fm Last: lst IfEmpty: e ).
    doFirstMiddle: fm Last: lst = ( doFirst: fm Middle: fm Last: lst IfEmpty: [] ).
    do: eb SeparatedBy: sb = (
      doFirst: [| :e. :k | eb value: e With: k ]
      MiddleLast: [| :e. :k | sb value. eb value: e With: k ]
      IfEmpty: [] ) }.
  { 'Category: searching'
    findFirst: eb IfPresent: fb IfAbsent: ab = (
      do: [| :v. :k | (eb value: v With: k) ifTrue: [ ^ fb value: v With: k ] ].
      ab value ).
    findFirst: eb IfPresent: fb = ( findFirst: eb IfPresent: fb IfAbsent: [ ^ self ] ).
    keyOf: x = ( keyOf: x IfAbsent: [ error: 'not found: ', x printString ] ).
    keyOf: x IfAbsent: ab = ( findFirst: [| :v | x = v ] IfPresent: [| :v. :k | k ] IfAbsent: ab ).
    occurrencesOf: x = ( | n <- 0 | do: [| :e | x = e ifTrue: [ n: n + 1 ] ]. n ).
    occurrencesOfEachElement = ( | d |
      d: dictionary copyRemoveAll.
      do: [| :e | d at: e Put: (d at: e IfAbsent: 0) + 1 ].
      d ) }.
  { 'Category: reducing'
    countHowMany: b = ( count: b ).
    reduceWith: b = ( reduceWith: b IfSingleton: [| :e | e ] IfEmpty: [ error: 'reducing an empty collection' ] ).
    reduceWith: b IfSingleton: sb = ( reduceWith: b IfSingleton: sb IfEmpty: [ error: 'reducing an empty collection' ] ).
    reduceWith: b IfSingleton: sb IfEmpty: mt = ( | r |
      size = 1 ifTrue: [ ^ sb value: first ].
      doFirst: [| :e | r: e ] MiddleLast: [| :e | r: (b value: r With: e) ] IfEmpty: [ r: mt value ].
      r ).
    product = ( reduceWith: [| :a. :b | a * b ] IfSingleton: [| :e | e ] IfEmpty: 1 ).
    "mean rounds for integers (it uses /~), as in the original; average always gives a float."
    mean = ( sum /~ size ).
    median = ( | v. m |
      v: copySort. m: size / 2.
      size odd ifTrue: [ v at: m ] False: [ (v at: m - 1) mean: (v at: m) ] ).
    harmonicMean = ( size / (inject: 0 Into: [| :s. :x | s + x inverse ]) ).
    rootMeanSquare = ( ((inject: 0 Into: [| :s. :x | s + x squared ]) asFloat / size) sqrt ).
    standardDeviation = ( | m |
      m: sum asFloat / size.
      ((inject: 0 Into: [| :s. :x | s + (x - m) squared ]) / (size - 1)) sqrt ).
    percentile: nth = ( | v | v: copySort. v at: (((size * nth) / 100) min: size - 1) ).
    dotProduct: c = ( | a. b. s <- 0 |
      a: asVector. b: c asVector.
      a with: b Do: [| :x. :y | s: s + (x * y) ].
      s ) }.
  { 'Category: set operations'
    union: c = ( copy addAll: c ).
    intersect: c = ( copyFilteredBy: [| :e | c includes: e ] ).
    difference: c = ( copyFilteredBy: [| :e | (c includes: e) not ] ) }.
  { 'Category: transforming'
    "These keep the kind of the receiver: a list gives a list, a string a string."
    copyFilteredBy: b = ( | r | r: copyRemoveAll.
      do: [| :e. :k | (b value: e With: k) ifTrue: [ r add: e ] ]. r ).
    copyMappedBy: b = ( | r | r: copyRemoveAll.
      do: [| :e. :k | r add: (b value: e With: k) ]. r ).
    filterBy: b = ( copyFilteredBy: b ).
    mapBy: b = ( copyMappedBy: b ).
    gather: b = ( | l | l: list copyRemoveAll. do: [| :e | l addAll: (b value: e) ]. l ).
    asSet = ( | s | s: set copyRemoveAll. do: [| :e | s add: e ]. s ).
    asOrderedSet = ( asSet ).
    asSequence = ( asList ).
    asDictionary = ( | d | d: dictionary copyRemoveAll. do: [| :e. :k | d at: k Put: e ]. d ).
    joinUsing: sep = ( | s <- '' | do: [| :e | s: s, e ] SeparatedBy: [ s: s, sep ]. s ) }.
  { 'Category: sorting'
    copySort = ( asVector sort ).
    copySortBy: b = ( asVector sortBy: b ).
    sortedDo: b = ( copySort do: b ).
    sortedBy: lt Do: b = ( (copySortBy: lt) do: b ) }
|).

traits indexable _AddSlots: (|
  { 'Category: accessing'
    at: i IfAbsent: b = ( ((i >= 0) && [ i < size ]) ifTrue: [ at: i ] False: [ b value ] ).
    firstIfAbsent: b = ( isEmpty ifTrue: [ b value ] False: [ first ] ).
    lastIfAbsent: b = ( isEmpty ifTrue: [ b value ] False: [ last ] ).
    first: n = ( copyFrom: 0 UpTo: (n min: size) ).
    last: n = ( copyFrom: (size - n) max: 0 ).
    keys = ( 0 to: size - 1 Collect: [| :i | i ] ) }.
  { 'Category: copying'
    copyFrom: a = ( copyFrom: a UpTo: size ).
    copyFrom: a Size: n = ( copyFrom: a UpTo: a + n ).
    copyAtMost: n = ( copyFrom: 0 UpTo: (n min: size) ).
    copyWithoutFirst = ( copyFrom: (1 min: size) ).
    copyWithoutLast = ( copyFrom: 0 UpTo: (size - 1) max: 0 ).
    copyWithoutPrefix: p = ( (p isPrefixOf: self) ifTrue: [ copyFrom: p size ] False: [ self ] ).
    copyWithoutSuffix: p = ( (p isSuffixOf: self) ifTrue: [ copyFrom: 0 UpTo: size - p size ] False: [ self ] ).
    copyReversed = ( reverse ) }.
  { 'Category: comparing'
    isPrefixOf: s = ( size > s size ifTrue: [ ^ false ].
      0 upTo: size Do: [| :i | (at: i) = (s at: i) ifFalse: [ ^ false ] ].
      true ).
    isSuffixOf: s = ( | off | off: s size - size.
      off < 0 ifTrue: [ ^ false ].
      0 upTo: size Do: [| :i | (at: i) = (s at: off + i) ifFalse: [ ^ false ] ].
      true ) }.
  { 'Category: enumerating'
    with: other ReverseDo: b = ( (size - 1) downTo: 0 Do: [| :i | b value: (at: i) With: (other at: i) ]. self ) }
|).

traits vector _AddSlots: (|
  { 'Category: copying'
    copyFrom: a UpTo: b = ( | r |
      ((a < 0) || [ (b > size) || [ a > b ] ]) ifTrue: [ ^ error: 'bad range ', a printString, ' to ', b printString ].
      r: copySize: b - a.
      a upTo: b Do: [| :i | r at: i - a Put: (at: i) ].
      r ).
    copyRemoveAll = ( copySize: 0 ).
    copyAddFirst: x = ( | r | r: copySize: size + 1.
      r at: 0 Put: x. 0 upTo: size Do: [| :i | r at: i + 1 Put: (at: i) ]. r ).
    copyFilteredBy: b = ( select: b ).
    copyMappedBy: b = ( collect: b ).
    union: c = ( self , c asVector ).
    copySort = ( sort ).
    "A stable merge sort; the receiver is left as it was."
    sortBy: lessThan = ( | src. dst. tmp. n. width <- 1. lo. mid. hi. i. j. k |
      n: size. src: copy. dst: copySize: n.
      [ width < n ] whileTrue: [
        lo: 0.
        [ lo < n ] whileTrue: [
          mid: (lo + width) min: n. hi: (lo + (2 * width)) min: n.
          i: lo. j: mid. k: lo.
          [ k < hi ] whileTrue: [
            ((i < mid) && [ (j >= hi) || [ (lessThan value: (src at: j) With: (src at: i)) not ] ])
              ifTrue: [ dst at: k Put: (src at: i). i: i + 1 ]
              False: [ dst at: k Put: (src at: j). j: j + 1 ].
            k: k + 1 ].
          lo: hi ].
        tmp: src. src: dst. dst: tmp.
        width: width * 2 ].
      src ) }.
  { 'Category: initializing'
    fillWith: x = ( 0 upTo: size Do: [| :i | at: i Put: x ]. self ).
    fillFrom: c = ( | i <- 0 | c do: [| :e | i < size ifTrue: [ at: i Put: e. i: i + 1 ] ]. self ) }
|).

traits list _AddSlots: (|
  { 'Category: adding'
    addFirst: x = ( add: x.
      (count - 1) downTo: 1 Do: [| :i | items at: i Put: (items at: i - 1) ].
      items at: 0 Put: x.
      self ).
    addAllFirst: c = ( c asVector reverseDo: [| :e | addFirst: e ]. self ) }.
  { 'Category: removing'
    remove: x = ( remove: x IfAbsent: [ error: 'not in the list: ', x printString ] ).
    remove: x IfAbsent: b = ( | i |
      i: indexOf: x.
      i < 0 ifTrue: [ ^ b value ].
      i + 1 upTo: count Do: [| :j | items at: j - 1 Put: (items at: j) ].
      count: count - 1.
      items at: count Put: nil.
      x ).
    removeAll: c = ( c do: [| :e | remove: e ]. self ).
    removeFirstIfAbsent: b = ( isEmpty ifTrue: [ b value ] False: [ removeFirst ] ).
    removeLastIfAbsent: b = ( isEmpty ifTrue: [ b value ] False: [ removeLast ] ) }.
  { 'Category: copying'
    , c = ( copy addAll: c ).
    copyFrom: a UpTo: b = ( | r |
      ((a < 0) || [ (b > count) || [ a > b ] ]) ifTrue: [ ^ error: 'bad range ', a printString, ' to ', b printString ].
      r: copyRemoveAll.
      a upTo: b Do: [| :i | r add: (items at: i) ].
      r ).
    reverse = ( | r | r: copyRemoveAll. reverseDo: [| :e | r add: e ]. r ).
    copySort = ( asVector sort asList ).
    copySortBy: b = ( (asVector sortBy: b) asList ) }
|).

traits dictionary _AddSlots: (|
  { 'Category: accessing'
    at: k IfAbsentPut: b = ( | i | i: keys indexOf: k.
      i >= 0 ifTrue: [ values at: i ] False: [ | v | v: b value. at: k Put: v. v ] ).
    if: k IsPresentDo: p = ( if: k IsPresentDo: p IfAbsentDo: [] ).
    if: k IsPresentDo: p IfAbsentDo: a = ( | i | i: keys indexOf: k.
      i >= 0 ifTrue: [ p value: (values at: i) ] False: [ a value ] ) }.
  { 'Category: adding'
    addAll: d = ( d do: [| :v. :k | at: k Put: v ]. self ) }.
  { 'Category: removing'
    removeKey: k IfAbsent: b = ( (includesKey: k) ifTrue: [ removeKey: k ] False: [ b value ] ).
    remove: v = ( remove: v IfAbsent: [ error: 'not in the dictionary: ', v printString ] ).
    remove: v IfAbsent: b = ( removeKey: (keyOf: v IfAbsent: [ ^ b value ]). v ).
    removeAll = ( keys: list copyRemoveAll. values: list copyRemoveAll. self ) }.
  { 'Category: transforming'
    copyFilteredBy: b = ( | r | r: copyRemoveAll.
      do: [| :v. :k | (b value: v With: k) ifTrue: [ r at: k Put: v ] ]. r ).
    copyMappedBy: b = ( | r | r: copyRemoveAll.
      do: [| :v. :k | r at: k Put: (b value: v With: k) ]. r ).
    asDictionary = ( copy ) }
|).

traits _AddSlots: (|
  set = (| parent* = traits collection |)
|).

globals _AddSlots: (|
  set = (| parent* = traits set. items <- list copyRemoveAll |)
|).

traits set _AddSlots: (|
  {} = 'Each element at most once, kept in the order added: set copyRemoveAll add: 3'.
  { 'Category: adding'
    add: x = ( (items includes: x) ifFalse: [ items add: x ]. self ).
    addAll: c = ( c do: [| :e | add: e ]. self ) }.
  { 'Category: removing'
    remove: x = ( remove: x IfAbsent: [ error: 'not in the set: ', x printString ] ).
    remove: x IfAbsent: b = ( items remove: x IfAbsent: [ ^ b value ]. x ).
    removeAll = ( items: list copyRemoveAll. self ) }.
  { 'Category: accessing'
    size = ( items size ).
    includes: x = ( items includes: x ) }.
  { 'Category: enumerating'
    do: b = ( items do: [| :e | b value: e With: e ]. self ) }.
  { 'Category: copying'
    copy = ( resend.copy items: items copy ).
    copyRemoveAll = ( copy removeAll ).
    asSet = ( copy ) }.
  printString = ( 'set(', ((items asVector collect: [| :e | e printString ]) joinUsing: ', '), ')' )
|).

globals _AddSlots: (|
  { 'Category: original names'
    "Our set, dictionary and list already keep insertion order and are indexable."
    orderedSet = set.
    orderedDictionary = dictionary.
    sequence = list }
|).

"---- strings"

traits string _AddSlots: (|
  { 'Category: comparing'
    <= s = ( (s < self) not ).
    >= s = ( (self < s) not ).
    max: s = ( self < s ifTrue: [ s ] False: [ self ] ).
    min: s = ( self < s ifTrue: [ self ] False: [ s ] ).
    compare: s IfLess: l Equal: e Greater: g = (
      self < s ifTrue: [ l value ] False: [ self = s ifTrue: [ e value ] False: [ g value ] ] ) }.
  { 'Category: characters'
    "A character is a string of size 1."
    isDigit = ( (size = 1) && [ '0123456789' includesSubstring: self ] ).
    isLetter = ( (size = 1) && [ asUppercase != asLowercase ] ).
    isUpperCase = ( isLetter && [ self = asUppercase ] ).
    isLowerCase = ( isLetter && [ self = asLowercase ] ).
    isVowel = ( (size = 1) && [ 'aeiouAEIOU' includesSubstring: self ] ).
    isWordSeparator = ( (size = 1) && [ ' \t\n\r' includesSubstring: self ] ).
    isLineBreak = ( (self = '\n') || [ self = '\r' ] ).
    isControlCharacter = ( (size = 1) && [ (asByte < 32) || [ asByte = 127 ] ] ).
    isPrintable = ( (size = 1) && [ isControlCharacter not ] ).
    succ = ( (asByte + 1) asCharacter ).
    pred = ( (asByte - 1) asCharacter ).
    successor = ( succ ).
    predecessor = ( pred ) }.
  { 'Category: casing'
    capitalizeAll = ( asUppercase ).
    uncapitalizeAll = ( asLowercase ).
    uncapitalize = ( isEmpty ifTrue: [ self ] False: [ (copyFrom: 0 UpTo: 1) asLowercase, (copyFrom: 1) ] ).
    isCapitalized = ( notEmpty && [ first isUpperCase ] ).
    isUncapitalized = ( isCapitalized not ) }.
  { 'Category: copying'
    copyRemoveAll = ( '' ).
    copyAddFirst: s = ( s asString, self ).
    copyAddLast: s = ( self, s asString ).
    copyFilteredBy: b = ( | s <- '' | do: [| :c. :i | (b value: c With: i) ifTrue: [ s: s, c ] ]. s ).
    copyMappedBy: b = ( | s <- '' | do: [| :c. :i | s: s, (b value: c With: i) asString ]. s ).
    replace: old With: new = ( | r <- ''. rest. i |
      old isEmpty ifTrue: [ ^ self ].
      rest: self.
      [ i: rest indexOfSubstring: old. i >= 0 ] whileTrue: [
        r: r, (rest copyFrom: 0 UpTo: i), new.
        rest: rest copyFrom: i + old size ].
      r, rest ) }.
  { 'Category: padding'
    shrinkwrapped = ( trim ).
    padOnLeft: n = ( | s | s: self. (n - size) timesRepeat: [ s: ' ', s ]. s ).
    padOnRight: n = ( | s | s: self. (n - size) timesRepeat: [ s: s, ' ' ]. s ).
    isAllWhiteSpace = ( allSatisfy: [| :c | c isWordSeparator ] ) }.
  { 'Category: lines'
    firstLine = ( lines isEmpty ifTrue: [ '' ] False: [ lines first ] ).
    lastLine = ( lines isEmpty ifTrue: [ '' ] False: [ lines last ] ).
    asTextLines = ( lines ) }.
  { 'Category: searching'
    findSubstring: s IfPresent: p IfAbsent: a = ( | i | i: indexOfSubstring: s.
      i < 0 ifTrue: [ a value ] False: [ p value: i ] ) }.
  { 'Category: tokenizing'
    splitOn: sep = ( split: sep ).
    asTokensSeparatedByCharactersIn: cs = ( | l. cur <- '' |
      l: list copyRemoveAll.
      do: [| :c | (cs includesSubstring: c)
        ifTrue: [ cur isEmpty ifFalse: [ l add: cur. cur: '' ] ]
        False: [ cur: cur, c ] ].
      cur isEmpty ifFalse: [ l add: cur ].
      l asVector ).
    asTokensSeparatedByWhiteSpace = ( asTokensSeparatedByCharactersIn: ' \t\n\r' ).
    asWords = ( asTokensSeparatedByWhiteSpace ) }.
  { 'Category: matching'
    "* matches any run of characters and ? any one character."
    matchesPattern: p = ( matchesPattern: p IgnoreCase: false ).
    matchesPattern: p IgnoreCase: ic = ( ic
      ifTrue: [ asLowercase matchFrom: 0 Pattern: p asLowercase At: 0 ]
      False: [ matchFrom: 0 Pattern: p At: 0 ] ).
    matchFrom: i Pattern: p At: j = (
      j = p size ifTrue: [ ^ i = size ].
      (p at: j) = '*' ifTrue: [
        i to: size Do: [| :k | (matchFrom: k Pattern: p At: j + 1) ifTrue: [ ^ true ] ].
        ^ false ].
      i = size ifTrue: [ ^ false ].
      (((p at: j) = '?') || [ (p at: j) = (at: i) ])
        ifTrue: [ matchFrom: i + 1 Pattern: p At: j + 1 ]
        False: [ false ] ) }.
  { 'Category: converting'
    asIntegerIfFail: b = ( _StringAsIntegerIfFail: b ).
    asFloatIfFail: b = ( _StringAsFloatIfFail: b ).
    hexAsInteger = ( hexAsIntegerIfFail: [ error: 'not a hexadecimal number: ', printString ] ).
    hexAsIntegerIfFail: fb = ( | n <- 0 |
      isEmpty ifTrue: [ ^ fb value ].
      do: [| :c. d | d: '0123456789abcdef' indexOfSubstring: c asLowercase.
        d < 0 ifTrue: [ ^ fb value ].
        n: (n * 16) + d ].
      n ).
    asSelector = ( self ).
    asSmallInteger = ( asInteger ) }.
  { 'Category: files'
    "On the desktop: 'notes.txt' asFileContents, 'notes.txt' setFileContentsTo: 'hi'."
    asFileContents = ( _FileContentsIfFail: [| :e | error: e ] ).
    asFileContentsIfFail: b = ( _FileContentsIfFail: b ).
    setFileContentsTo: text = ( _SetFileContents: text asString IfFail: [| :e | error: e ] ).
    fileExists = ( _FileExists ) }.
  { 'Category: copying with ellipses'
    copyAtMostWithEllipsis: n = ( size <= n ifTrue: [ self ] False: [ (copyFrom: 0 UpTo: (n - 3) max: 0), '...' ] ).
    leadingWhiteSpace = ( | n <- 0 | do: [| :c | c isWordSeparator ifFalse: [ ^ n ]. n: n + 1 ]. n ).
    findSubstring: s StartingAt: i IfPresent: p IfAbsent: a = ( | j |
      j: (copyFrom: i) indexOfSubstring: s.
      j < 0 ifTrue: [ a value ] False: [ p value: i + j ] ).
    asTokensSeparatedBySubstring: sep = ( (split: sep) reject: [| :t | t isEmpty ] ) }.
  { 'Category: sending'
    "'+' sendTo: 3 With: 4 sends the message named by the string."
    sendTo: r = ( r _Perform: self ).
    sendTo: r With: a = ( r _Perform: self With: a ).
    sendTo: r With: a With: b = ( r _Perform: self With: a With: b ).
    sendTo: r With: a With: b With: c = ( r _Perform: self With: a With: b With: c ).
    sendTo: r WithArguments: v = (
      v size = 0 ifTrue: [ ^ sendTo: r ].
      v size = 1 ifTrue: [ ^ sendTo: r With: (v at: 0) ].
      v size = 2 ifTrue: [ ^ sendTo: r With: (v at: 0) With: (v at: 1) ].
      v size = 3 ifTrue: [ ^ sendTo: r With: (v at: 0) With: (v at: 1) With: (v at: 2) ].
      error: 'sendTo:WithArguments: takes at most 3 arguments' ) }.
  { 'Category: evaluating'
    evalIfFail: b = ( [ eval ] onError: [| :m | b value: m ] ) }.
  { 'Category: translating'
    "Self source to JavaScript: '3 + 4' asJavaScript. asWebApp makes a whole web page
     (runtime, starting world and morphs) that runs the program in any browser."
    asJavaScript = ( _StringAsJavaScript ).
    asWebApp = ( asWebAppTitled: 'A Self app' ).
    asWebAppTitled: t = ( _StringAsWebApp: t ) }.
  storeStringIfFail: b = ( printString )
|).

"---- blocks"

traits block _AddSlots: (|
  { 'Category: control'
    assert = ( value ifFalse: [ error: 'assertion failed' ]. self ).
    assert: msg = ( value ifFalse: [ error: 'assertion failed: ', msg ]. self ).
    "[| :exit | ... exit value ... ] loopExit runs until the block calls exit."
    loopExit = ( [ value: [ ^ nil ] ] loop ).
    loopExitValue = ( [ value: [| :x | ^ x ] ] loop ).
    exit = ( value: [ ^ nil ] ).
    exitValue = ( value: [| :x | ^ x ] ).
    "[| :continue | ... continue value ... ] loopContinue starts the next round early."
    loopContinue = ( [ exit ] loop ).
    loopExitContinue = ( [ loopExitContinueOnce: [ ^ nil ] ] loop ).
    loopExitValueContinue = ( [ loopExitContinueOnce: [| :x | ^ x ] ] loop ).
    loopExitContinueOnce: e = ( value: e With: [ ^ nil ] ) }.
  { 'Category: unwind protect'
    "[ ... ] onReturn: [ cleanup ] runs the cleanup however the block is left."
    onReturn: b = ( _OnReturn: b ).
    onNonLocalReturn: b = ( _OnNonLocalReturn: b ) }.
  { 'Category: timing'
    realTime = ( msecToRun ).
    cpuTime = ( msecToRun ) }
|).

"---- points and rectangles"

traits point _AddSlots: (|
  { 'Category: copying'
    copyX: a Y: b = ( (copy x: a) y: b ).
    swap = ( copyX: y Y: x ) }.
  { 'Category: arithmetic'
    negate = ( copyX: x negated Y: y negated ).
    negated = ( negate ).
    max: p = ( | q | q: p asPoint. copyX: (x max: q x) Y: (y max: q y) ).
    min: p = ( | q | q: p asPoint. copyX: (x min: q x) Y: (y min: q y) ).
    half = ( self / 2 ).
    square = ( self * self ).
    inverse = ( copyX: x inverse Y: y inverse ).
    mean: p = ( (self + p) / 2 ).
    addX: n = ( copyX: x + n Y: y ).
    addY: n = ( copyX: x Y: y + n ).
    subtractX: n = ( copyX: x - n Y: y ).
    subtractY: n = ( copyX: x Y: y - n ).
    distanceTo: p = ( dist: p ).
    manhattanDistanceTo: p = ( (x - p x) abs + (y - p y) abs ).
    dotProduct: p = ( (x * p x) + (y * p y) ).
    crossProduct: p = ( (x * p y) - (y * p x) ).
    aspectRatio = ( x asFloat / y ) }.
  { 'Category: geometry'
    translateBy: p = ( self + p ).
    xAxisReflect = ( copyX: x Y: y negated ).
    yAxisReflect = ( copyX: x negated Y: y ).
    theta = ( y arcTan: x ).
    thetaDegrees = ( (theta * 180.0) / 1 pi ).
    rotateBy: radians = ( | rr. t | rr: r. t: theta + radians. (rr * t cos) @ (rr * t sin) ).
    rho: rr Theta: t = ( (rr * t cos) @ (rr * t sin) ).
    normalize = ( self / r ).
    alignToGrid: g = ( | q | q: g asPoint. ((x / q x) * q x) @ ((y / q y) * q y) ).
    restrictTo: rect = ( ((x max: rect left) min: rect right) @ ((y max: rect top) min: rect bottom) ).
    # p = ( rectangle from: self To: p ).
    ## s = ( rectangle from: self To: self + s ).
    asRectangle = ( self # self ) }.
  { 'Category: converting'
    asPoint = ( self ).
    rounded = ( copyX: x rounded Y: y rounded ).
    round = ( rounded ).
    truncate = ( copyX: x truncate Y: y truncate ).
    floor = ( copyX: x floor Y: y floor ).
    ceil = ( copyX: x ceil Y: y ceil ).
    asInteger = ( copyX: x asInteger Y: y asInteger ).
    asFloat = ( copyX: x asFloat Y: y asFloat ) }.
  storeStringIfFail: b = ( '(', printString, ')' )
|).

traits _AddSlots: (|
  rectangle = (| parent* = traits clonable |)
|).

globals _AddSlots: (|
  rectangle = (| parent* = traits rectangle. origin <- 0 @ 0. corner <- 0 @ 0 |)
|).

traits rectangle _AddSlots: (|
  {} = 'Rectangles from a top-left origin to a bottom-right corner: (0@0) # (10@20)'.
  { 'Category: creating'
    from: p1 To: p2 = ( (copy origin: (p1 min: p2)) corner: (p1 max: p2) ) }.
  { 'Category: accessing'
    left = ( origin x ).
    top = ( origin y ).
    right = ( corner x ).
    bottom = ( corner y ).
    left: n = ( copy origin: n @ top ).
    top: n = ( copy origin: left @ n ).
    right: n = ( copy corner: n @ bottom ).
    bottom: n = ( copy corner: right @ n ).
    topLeft = ( origin ).
    topRight = ( right @ top ).
    bottomLeft = ( left @ bottom ).
    bottomRight = ( corner ).
    width = ( right - left ).
    height = ( bottom - top ).
    extent = ( corner - origin ).
    size = ( extent ).
    area = ( width * height ).
    center = ( (origin + corner) / 2 ).
    topCenter = ( center x @ top ).
    bottomCenter = ( center x @ bottom ).
    leftCenter = ( left @ center y ).
    rightCenter = ( right @ center y ).
    center: c Size: s = ( from: c - (s / 2) To: c - (s / 2) + s ) }.
  { 'Category: testing'
    isRectangle = true.
    includes: p = ( (p x >= left) && [ (p x <= right) && [ (p y >= top) && [ p y <= bottom ] ] ] ).
    intersects: r = ( (left < r right) && [ (r left < right) && [ (top < r bottom) && [ r top < bottom ] ] ] ).
    enclosesOrEquals: r = ( (includes: r topLeft) && [ includes: r bottomRight ] ).
    encloses: r = ( (enclosesOrEquals: r) && [ self != r ] ).
    = r = ( (r _Eq: self) || [ r isRectangle && [ (origin = r origin) && [ corner = r corner ] ] ] ).
    hash = ( origin x hash + corner y hash ) }.
  { 'Category: combining'
    intersect: r = ( | l. t. rr. b |
      l: left max: r left. rr: right min: r right.
      t: top max: r top. b: bottom min: r bottom.
      ((l >= rr) || [ t >= b ]) ifTrue: [ ^ (0 @ 0) # (0 @ 0) ].
      (l @ t) # (rr @ b) ).
    union: r = ( ((left min: r left) @ (top min: r top)) # ((right max: r right) @ (bottom max: r bottom)) ) }.
  { 'Category: moving'
    translateBy: p = ( (origin + p) # (corner + p) ).
    translateTo: p = ( p ## extent ).
    indent: n = ( (origin + n) # (corner - n) ).
    extend: n = ( (origin - n) # (corner + n) ) }.
  printString = ( '(', origin printString, ') # (', corner printString, ')' )
|).

"---- equality, ordering and inserting for sequences"

traits collection _AddSlots: (|
  { 'Category: comparing'
    "Same size and equal elements in the same order (as the original's equalsCollection:)."
    equalsCollection: c = ( | a. b |
      size = c size ifFalse: [ ^ false ].
      a: asVector. b: c asVector.
      0 upTo: a size Do: [| :i | (a at: i) = (b at: i) ifFalse: [ ^ false ] ].
      true ).
    contentsHash = ( | h <- 0 | do: [| :e | h: ((h * 31) + e hash) % 1073741823 ]. h ) }
|).

traits vector _AddSlots: (|
  { 'Category: comparing'
    "Vectors with equal elements are equal, as in the original."
    = v = ( (v _Eq: self) || [ v isVector && [ equalsCollection: v ] ] ).
    hash = ( contentsHash ) }
|).

traits list _AddSlots: (|
  isList = true.
  { 'Category: comparing'
    = l = ( (l _Eq: self) || [ l isList && [ equalsCollection: l ] ] ).
    hash = ( contentsHash ) }.
  { 'Category: inserting'
    insert: x At: i = ( add: x.
      (count - 1) downTo: i + 1 Do: [| :j | items at: j Put: (items at: j - 1) ].
      items at: i Put: x.
      self ).
    insert: x BeforeElementSatisfying: b IfAbsent: ab = (
      findFirst: [| :e | b value: e ] IfPresent: [| :e. :i | ^ insert: x At: i ] IfAbsent: [ ^ ab value ] ).
    insert: x AfterElementSatisfying: b IfAbsent: ab = (
      findFirst: [| :e | b value: e ] IfPresent: [| :e. :i | ^ insert: x At: i + 1 ] IfAbsent: [ ^ ab value ] ).
    insertAll: c BeforeElementSatisfying: b IfAbsent: ab = ( | i |
      i: (findFirst: [| :e | b value: e ] IfPresent: [| :e. :k | k ] IfAbsent: [ ^ ab value ]).
      c asVector reverseDo: [| :e | insert: e At: i ]. self ).
    insertAll: c AfterElementSatisfying: b IfAbsent: ab = ( | i |
      i: (findFirst: [| :e | b value: e ] IfPresent: [| :e. :k | k ] IfAbsent: [ ^ ab value ]).
      c asVector reverseDo: [| :e | insert: e At: i + 1 ]. self ) }
|).

traits indexable _AddSlots: (|
  { 'Category: comparing'
    "Sequences compare element by element, like words in a dictionary."
    < s = ( 0 upTo: (size min: s size) Do: [| :i |
        (at: i) < (s at: i) ifTrue: [ ^ true ].
        (s at: i) < (at: i) ifTrue: [ ^ false ] ].
      size < s size ).
    > s = ( s < self ).
    <= s = ( (s < self) not ).
    >= s = ( (self < s) not ).
    max: s = ( self < s ifTrue: [ s ] False: [ self ] ).
    min: s = ( self < s ifTrue: [ self ] False: [ s ] ).
    compare: s IfLess: l Equal: e Greater: g = (
      self < s ifTrue: [ l value ] False: [ s < self ifTrue: [ g value ] False: [ e value ] ] ) }
|).

traits set _AddSlots: (|
  { 'Category: accessing'
    includesKey: x = ( includes: x ).
    keys = ( copy ).
    at: x = ( (includes: x) ifTrue: [ x ] False: [ error: 'not in the set: ', x printString ] ).
    at: x IfAbsent: b = ( (includes: x) ifTrue: [ x ] False: [ b value ] ) }.
  { 'Category: removing'
    removeKey: x = ( remove: x ).
    removeKey: x IfAbsent: b = ( remove: x IfAbsent: b ).
    removeFirst = ( | x | x: first. remove: x. x ).
    removeFirstIfAbsent: b = ( isEmpty ifTrue: [ b value ] False: [ removeFirst ] ).
    removeAll: c = ( c do: [| :e | remove: e ]. self ) }.
  { 'Category: comparing'
    = s = ( (s _Eq: self) || [ (s _SlotLookup: 'items' IfFail: nil) notNil && [ (size = s size) && [ includesAll: s ] ] ] ).
    hash = ( size ) }
|).

traits rectangle _AddSlots: (|
  { 'Category: arithmetic'
    "A rectangle is a pair of points in the original: arithmetic applies to both corners."
    + p = ( (origin + p) # (corner + p) ).
    - p = ( (origin - p) # (corner - p) ).
    * k = ( (origin * k) # (corner * k) ).
    / k = ( (origin / k) # (corner / k) ).
    rounded = ( origin rounded # corner rounded ).
    truncate = ( origin truncate # corner truncate ) }.
  { 'Category: pair'
    x = ( origin ).
    y = ( corner ).
    copyX: a Y: b = ( a # b ).
    topLeft: p = ( p # corner ).
    bottomRight: p = ( origin # p ).
    topRight: p = ( (left @ p y) # (p x @ bottom) ).
    bottomLeft: p = ( (p x @ top) # (right @ p y) ).
    center: c = ( translateBy: c - center ).
    restrictTo: r = ( translateTo: (origin restrictTo: ((r origin) # ((r corner - extent) max: r origin))) ) }.
  { 'Category: stretching'
    "shrink moves an edge inwards, stretch moves it outwards; each gives a new rectangle."
    shrinkLeft: n = ( (origin addX: n) # corner ).
    shrinkTop: n = ( (origin addY: n) # corner ).
    shrinkRight: n = ( origin # (corner subtractX: n) ).
    shrinkBottom: n = ( origin # (corner subtractY: n) ).
    shrinkTopLeft: p = ( (origin + p) # corner ).
    shrinkBottomRight: p = ( origin # (corner - p) ).
    stretchLeft: n = ( (origin subtractX: n) # corner ).
    stretchTop: n = ( (origin subtractY: n) # corner ).
    stretchRight: n = ( origin # (corner addX: n) ).
    stretchBottom: n = ( origin # (corner addY: n) ).
    stretchTopLeft: p = ( (origin - p) # corner ).
    stretchBottomRight: p = ( origin # (corner + p) ) }.
  { 'Category: ui'
    pointClosestToPoint: p = ( ((p x max: left) min: right) @ ((p y max: top) min: bottom) ) }
|).

traits point _AddSlots: (|
  { 'Category: arithmetic'
    /+ p = ( | q | q: p asPoint. (x /+ q x) @ (y /+ q y) ).
    /- p = ( | q | q: p asPoint. (x /- q x) @ (y /- q y) ).
    /~ p = ( | q | q: p asPoint. (x /~ q x) @ (y /~ q y) ).
    /= p = ( | q | q: p asPoint. (x /= q x) @ (y /= q y) ) }
|).

traits collection _AddSlots: (|
  { 'Category: accessing'
    selectAtRandom = ( selectAtRandom: [ error: 'the collection is empty' ] ).
    selectAtRandom: fb = ( | n |
      isEmpty ifTrue: [ ^ fb value ].
      n: random integer: size.
      do: [| :e | n = 0 ifTrue: [ ^ e ]. n: n - 1 ].
      fb value ).
    selectAtRandom: fb IfFail: b = ( selectAtRandom: b ) }.
  { 'Category: copying'
    copyContaining: c = ( copyRemoveAll addAll: c ) }
|).

traits indexable _AddSlots: (|
  { 'Category: accessing'
    keyAt: i = ( i ).
    keyAt: i IfAbsent: b = ( ((i >= 0) && [ i < size ]) ifTrue: [ i ] False: [ b value ] ).
    firstKey = ( 0 ).
    lastKey = ( size - 1 ) }.
  { 'Category: enumerating'
    randomlyDo: b = ( | order. j. t |
      order: (0 to: size - 1 Collect: [| :i | i ]).
      (size - 1) downTo: 1 Do: [| :i |
        j: random integer: i + 1.
        t: order at: i. order at: i Put: (order at: j). order at: j Put: t ].
      order do: [| :i | b value: (at: i) With: i ].
      self ) }
|).

traits vector _AddSlots: (|
  { 'Category: searching'
    "Search for a run of elements, as strings search for substrings."
    findSubstring: s StartingAt: from IfPresent: p IfAbsent: a = (
      from upTo: (size - s size) + 1 Do: [| :i. ok |
        ok: true.
        0 upTo: s size Do: [| :k | ((at: i + k) = (s at: k)) ifFalse: [ ok: false ] ].
        ok ifTrue: [ ^ p value: i ] ].
      a value ).
    findSubstring: s IfPresent: p IfAbsent: a = ( findSubstring: s StartingAt: 0 IfPresent: p IfAbsent: a ).
    includesSubstring: s = ( findSubstring: s IfPresent: [ true ] IfAbsent: [ false ] ) }.
  { 'Category: split and join'
    splitOn: sep = ( | parts. start. i |
      parts: list copyRemoveAll. start: 0. i: 0.
      sep isEmpty ifTrue: [ ^ (vector copySize: 0) copyAddLast: self ].
      [ i <= (size - sep size) ] whileTrue: [
        findSubstring: sep StartingAt: i
          IfPresent: [| :at | parts add: (copyFrom: start UpTo: at). start: at + sep size. i: start ]
          IfAbsent: [ i: size ] ].
      parts add: (copyFrom: start).
      parts asVector ) }.
  { 'Category: substitution'
    replace: old With: new = ( | parts. r |
      parts: splitOn: old.
      r: parts first.
      1 upTo: parts size Do: [| :k | r: r , new , (parts at: k) ].
      r ).
    replaceAllSatisfying: b With: x = ( 0 upTo: size Do: [| :i | (b value: (at: i)) ifTrue: [ at: i Put: x ] ]. self ).
    replaceFrom: a UpTo: e With: c = ( | v | v: c asVector.
      a upTo: e Do: [| :i | at: i Put: (v at: i - a) ].
      self ) }
|).

"---- saving objects as Self source"

globals _AddSlots: (|
  fileOut = (|
    {} = 'Writes objects as Self source. fileOut changes is everything you added to the world; fileOut object: x is one object.'.
    parent* = traits oddball.
    changes = ( _FileOutChanges ).
    changesTo: path = ( path setFileContentsTo: changes ).
    object: x = ( x _FileOut ).
    printString = 'fileOut' |)
|).

"---------------------------------------------------------------- loading original Self files

 Original Self source files describe the world with statements such as
   bootstrap addSlotsTo: bootstrap stub -> 'globals' -> 'true' -> () From: ( | ... | )
 These objects let such files be loaded: a stub path names an object by the
 slots leading to it from the lobby, creating empty objects that are missing."

traits _AddSlots: (|
  bootstrapPath = (| parent* = traits clonable |)
|).

traits bootstrapPath _AddSlots: (|
  {} = 'A path of slot names from the lobby, built with ->; ending it with () finds the object.'.
  -> n = ( _BootstrapArrow: n ).
  printString = ( 'a bootstrap path' )
|).

globals _AddSlots: (|
  bootstrap = (|
    {} = 'Understands the statements of original Self source files.'.
    parent* = traits oddball.
    stub = ( _BootstrapStub ).
    addSlotsTo: o From: lit = ( o _AddSlots: lit. o ).
    addSlotsIfAbsentTo: o From: lit = ( o _AddSlotsIfAbsent: lit. o ).
    define: o As: lit = ( o _Define: lit. o ).
    define: o ToBe: lit = ( o _Define: lit. o ).
    setObjectAnnotationOf: o From: annotation = ( o ).
    remove: name From: o = ( o _RemoveSlot: name IfFail: [ o ]. o ).
    read: name From: dir = ( _LoadSubpart: name From: dir ).
    "The folder of the file being loaded, or nil (in the browser, or for typed code)."
    sourceFolder = ( _SourceFolder )
  |)
|).

traits _AddSlots: (|
  module = (|
    {} = 'Shared behaviour of module objects created by original Self source files.'.
    parent* = traits clonable.
    postFileIn = ( self ).
    printString = ( 'a module' ) |)
|).

globals _AddSlots: (|
  modules = (|
    {} = 'One object per loaded original Self source file.'.
    parent* = traits oddball |)
|).

modules _AddSlots: (|
  init = (| parent* = traits module.
            comment <- ''. directory <- ''. fileInTimeString <- ''. myComment <- ''.
            revision <- ''. subpartNames <- '' |)
|).
