"A sketch pad, written in Self.

 Run it as an app:      self-world --app examples/sketch.self
 or in the desktop:     World > Run a Self file as an app...

 Click the canvas to put down dots; with 'Join the dots' on, each new dot
 is joined to the last by a line morph. The canvas is told where it was
 clicked with clickAt: (a point inside the canvas)."

lobby _AddSlots: (|
  sketchApp = (|
    {} = 'A canvas to click on. Send me open.'.
    parent* = traits clonable.
    canvas <- nil.
    colors <- nil.
    size <- nil.
    joined <- nil.
    last <- nil.

    { 'Category: building'
      open = ( | w. row |
        world title: 'Sketch'.
        canvas: canvasMorph copy.
        canvas extent: 440 @ 300.
        canvas _AddSlots: (| app <- nil. clickAt: p = ( app dotAt: p ) |).
        canvas app: self.

        colors: dropDownMorph copyChoices: ('teal' & 'tomato' & 'gold' & 'blue' & 'pink' & 'text') asVector.
        colors width: 110.
        joined: checkboxMorph copyLabel: 'Join the dots'.
        joined checked: true.
        size: sliderMorph copyMin: 4 Max: 40 Value: 14.

        row: rowMorph copy.
        row addMorph: colors.
        row addMorph: joined.
        row addMorph: (buttonMorph copyLabel: 'Clear' Action: [ clear ]).

        w: windowMorph copyTitle: 'Sketch: click the canvas'.
        w addMorph: row.
        w addMorph: size.
        w addMorph: canvas.
        w extent: 480 @ 420.
        w position: 60 @ 40.
        w openInWorld ) }.

    { 'Category: drawing'
      dotAt: p = ( | d. s. l |
        s: size value.
        (last notNil && [ joined checked ]) ifTrue: [
          l: lineMorph copyFrom: last To: p.
          l color: colors selected.
          l lineWidth: ((s / 4) max: 1).
          canvas addMorph: l ].
        d: circleMorph copy.
        d extent: s @ s.
        d color: colors selected.
        d position: p - ((s / 2) @ (s / 2)).
        canvas addMorph: d.
        last: p ).
      clear = ( canvas morphs do: [| :m | m delete ]. last: nil ) }
  |)
|).

sketchApp open.
