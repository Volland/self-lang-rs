"Morphic for Self World, written in Self.

 Interfaces are ordinary Self objects called morphs, as in the original
 Self and in Smalltalk. A morph's slots (x, y, width, height, color, text...)
 are read live by the desktop every frame, so changing a slot - from code or
 from an outliner - changes what you see at once. Put a morph on the screen
 with openInWorld; give it a step method and send startStepping to animate it."

traits _AddSlots: (|
  morph = (| parent* = traits clonable |)
|).

globals _AddSlots: (|
  morph = (| parent* = traits morph.
             x <- 40. y <- 40. width <- 120. height <- 80.
             color <- 'accent'.
             owner <- nil.
             isStepping <- false.
             submorphs <- list copyRemoveAll |)
|).

traits morph _AddSlots: (|
  {} = 'Shared behaviour of every morph: structure, geometry, stepping and events.'.
  morphKind = 'rectangle'.
  { 'Category: copying'
    copy = ( | c |
      c: resend.copy.
      c owner: nil.
      c submorphs: list copyRemoveAll.
      submorphs do: [| :m | c addMorph: m copy ].
      c ) }.
  { 'Category: structure'
    addMorph: m = ( m delete. submorphs add: m. m owner: self. self ).
    add: m = ( addMorph: m ).
    removeMorph: m = ( submorphs: (submorphs reject: [| :e | e == m ]) asList. m owner: nil. self ).
    morphs = ( submorphs asVector ).
    delete = ( owner isNil ifFalse: [ owner removeMorph: self ]. self ).
    openInWorld = ( world addMorph: self. self ).
    open = ( openInWorld ).
    isWorld = false }.
  { 'Category: geometry'
    position = ( x @ y ).
    position: p = ( x: p x. y: p y ).
    moveTo: p = ( position: p ).
    moveBy: p = ( x: x + p x. y: y + p y ).
    extent = ( width @ height ).
    extent: p = ( width: p x. height: p y ).
    center = ( (x + (width / 2)) @ (y + (height / 2)) ) }.
  { 'Category: stepping'
    step = ( self ).
    startStepping = ( isStepping: true. self ).
    stopStepping = ( isStepping: false. self ) }.
  { 'Category: events'
    click = ( self ).
    changed = ( self ) }.
  printString = ( 'a ', morphKind, ' morph' )
|).

traits _AddSlots: (|
  rectangleMorph = (| parent* = traits morph |).
  circleMorph = (| parent* = traits morph. morphKind = 'circle' |).
  labelMorph = (| parent* = traits morph. morphKind = 'label' |).
  buttonMorph = (| parent* = traits morph. morphKind = 'button' |).
  textFieldMorph = (| parent* = traits morph. morphKind = 'textField' |).
  sliderMorph = (| parent* = traits morph. morphKind = 'slider' |).
  checkboxMorph = (| parent* = traits morph. morphKind = 'checkbox' |).
  rowMorph = (| parent* = traits morph. morphKind = 'row' |).
  columnMorph = (| parent* = traits morph. morphKind = 'column' |).
  canvasMorph = (| parent* = traits morph. morphKind = 'canvas' |).
  windowMorph = (| parent* = traits morph. morphKind = 'window' |)
|).

traits labelMorph _AddSlots: (|
  copyText: t = ( copy text: t )
|).

traits buttonMorph _AddSlots: (|
  {} = 'A button. Its action block runs when it is clicked (it may take the button as argument).'.
  copyLabel: l Action: b = ( (copy label: l) action: b ).
  click = ( action isNil ifFalse: [ action value: self ]. self )
|).

traits textFieldMorph _AddSlots: (|
  {} = 'A one-line text field. onChange is called with the new contents and the field.'.
  copyContents: s = ( copy contents: s ).
  changed = ( onChange isNil ifFalse: [ onChange value: contents With: self ]. self )
|).

traits sliderMorph _AddSlots: (|
  {} = 'A slider from min to max. onChange is called with the new value and the slider.'.
  copyMin: a Max: b Value: v = ( ((copy min: a) max: b) value: v ).
  changed = ( onChange isNil ifFalse: [ onChange value: value With: self ]. self )
|).

traits checkboxMorph _AddSlots: (|
  copyLabel: l = ( copy label: l ).
  changed = ( onChange isNil ifFalse: [ onChange value: checked With: self ]. self )
|).

traits windowMorph _AddSlots: (|
  {} = 'A window on the desktop. Its submorphs are laid out top to bottom.'.
  copyTitle: t = ( copy title: t ).
  close = ( delete ).
  openInWorld = ( | n |
    "windows still at the prototype's place are cascaded so they don't hide each other"
    ((x = windowMorph x) && [ y = windowMorph y ]) ifTrue: [
      n: world morphs count: [| :m | m morphKind = 'window' ].
      position: (((world width - 460) max: windowMorph x) - (40 * (n % 6))) @ (70 + (44 * (n % 6))) ].
    resend.openInWorld )
|).

globals _AddSlots: (|
  { 'Category: ui'
  rectangleMorph = morph copy _AddSlots: (| parent* = traits rectangleMorph |).
  circleMorph = morph copy _AddSlots: (| parent* = traits circleMorph |).
  labelMorph = morph copy _AddSlots: (| parent* = traits labelMorph. text <- 'Label'. fontSize <- 16 |).
  buttonMorph = morph copy _AddSlots: (| parent* = traits buttonMorph. label <- 'Button'. action <- nil |).
  textFieldMorph = morph copy _AddSlots: (| parent* = traits textFieldMorph. contents <- ''. onChange <- nil |).
  sliderMorph = morph copy _AddSlots: (| parent* = traits sliderMorph. value <- 50. min <- 0. max <- 100. onChange <- nil |).
  checkboxMorph = morph copy _AddSlots: (| parent* = traits checkboxMorph. label <- 'Check'. checked <- false. onChange <- nil |).
  rowMorph = morph copy _AddSlots: (| parent* = traits rowMorph |).
  columnMorph = morph copy _AddSlots: (| parent* = traits columnMorph |).
  canvasMorph = morph copy _AddSlots: (| parent* = traits canvasMorph |).
  windowMorph = morph copy _AddSlots: (| parent* = traits windowMorph. title <- 'Window' |) }
|).

circleMorph extent: 60 @ 60.
labelMorph color: 'text'.
textFieldMorph width: 180.
sliderMorph width: 180.
canvasMorph extent: 320 @ 200.
canvasMorph color: 'background'.
windowMorph extent: 280 @ 160.
windowMorph position: 140 @ 120.

globals _AddSlots: (|
  world = (|
    {} = 'The world: every morph on the desktop is one of my submorphs. Try  circleMorph copy openInWorld'.
    parent* = traits morph.
    morphKind = 'world'.
    x <- 0. y <- 0. width <- 800. height <- 600.
    color <- 'none'.
    owner <- nil.
    isStepping <- false.
    submorphs <- list copyRemoveAll.
    isWorld = true.
    delete = ( self ).
    clear = ( submorphs do: [| :m | m owner: nil ]. submorphs: list copyRemoveAll. self ).
    printString = 'the world' |)
|).
