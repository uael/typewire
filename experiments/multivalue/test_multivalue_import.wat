;; Hand-crafted WAT to test: can a JS host function return multiple externrefs?
;; Per the WebAssembly JS API spec, if a host function's type has multiple results,
;; the engine should call @@iterator on the JS return value and destructure it.

(module
  ;; Import a JS function that returns 2 externrefs
  (import "env" "get_pair" (func $get_pair (result externref externref)))

  ;; Import a JS function that takes 1 externref and returns 3 externrefs
  (import "env" "destruct3" (func $destruct3 (param externref) (result externref externref externref)))

  ;; A table to store externrefs
  (table $refs 10 externref)

  ;; Export: call get_pair, store both results in the table
  (func (export "test_get_pair")
    (local $a externref)
    (local $b externref)
    ;; call get_pair — should return 2 externrefs on the stack
    (call $get_pair)
    (local.set $b)  ;; second value (top of stack)
    (local.set $a)  ;; first value
    ;; Store in table for JS to read back
    (table.set $refs (i32.const 0) (local.get $a))
    (table.set $refs (i32.const 1) (local.get $b))
  )

  ;; Export: call destruct3, store all 3 results
  (func (export "test_destruct3") (param $obj externref)
    (local $a externref)
    (local $b externref)
    (local $c externref)
    (call $destruct3 (local.get $obj))
    (local.set $c)
    (local.set $b)
    (local.set $a)
    (table.set $refs (i32.const 0) (local.get $a))
    (table.set $refs (i32.const 1) (local.get $b))
    (table.set $refs (i32.const 2) (local.get $c))
  )

  ;; Export: read a value from the table
  (func (export "get_result") (param $idx i32) (result externref)
    (table.get $refs (local.get $idx))
  )

  (export "refs" (table $refs))
)
