(module
  ;; Array-returning destruct (single JsValue return, then .get() per field)
  (import "env" "destruct_arr" (func $destruct_arr (param externref) (result externref)))
  (import "env" "arr_get" (func $arr_get (param externref i32) (result externref)))

  ;; Multi-value 3-return (array)
  (import "env" "destruct_mv3" (func $destruct_mv3 (param externref) (result externref externref externref)))

  ;; Multi-value 3-return (generator)
  (import "env" "destruct_mv3_gen" (func $destruct_mv3_gen (param externref) (result externref externref externref)))

  ;; Multi-value 3-return (destructured params)
  (import "env" "destruct_mv3_destr" (func $destruct_mv3_destr (param externref) (result externref externref externref)))

  ;; Per-field getters
  (import "env" "get_a" (func $get_a (param externref) (result externref)))
  (import "env" "get_b" (func $get_b (param externref) (result externref)))
  (import "env" "get_c" (func $get_c (param externref) (result externref)))

  ;; Reflect.get (each builds its own key string)
  (import "env" "reflect_get_a" (func $reflect_get_a (param externref) (result externref)))
  (import "env" "reflect_get_b" (func $reflect_get_b (param externref) (result externref)))
  (import "env" "reflect_get_c" (func $reflect_get_c (param externref) (result externref)))

  (table $refs 10 externref)

  ;; Approach 1: Array destruct then get
  (func (export "bench_array") (param $obj externref)
    (local $arr externref)
    (local.set $arr (call $destruct_arr (local.get $obj)))
    (table.set $refs (i32.const 0) (call $arr_get (local.get $arr) (i32.const 0)))
    (table.set $refs (i32.const 1) (call $arr_get (local.get $arr) (i32.const 1)))
    (table.set $refs (i32.const 2) (call $arr_get (local.get $arr) (i32.const 2)))
  )

  ;; Approach 2: Multi-value
  (func (export "bench_multivalue") (param $obj externref)
    (local $a externref) (local $b externref) (local $c externref)
    (call $destruct_mv3 (local.get $obj))
    (local.set $c) (local.set $b) (local.set $a)
    (table.set $refs (i32.const 0) (local.get $a))
    (table.set $refs (i32.const 1) (local.get $b))
    (table.set $refs (i32.const 2) (local.get $c))
  )

  ;; Approach 2b: Multi-value (generator)
  (func (export "bench_multivalue_gen") (param $obj externref)
    (local $a externref) (local $b externref) (local $c externref)
    (call $destruct_mv3_gen (local.get $obj))
    (local.set $c) (local.set $b) (local.set $a)
    (table.set $refs (i32.const 0) (local.get $a))
    (table.set $refs (i32.const 1) (local.get $b))
    (table.set $refs (i32.const 2) (local.get $c))
  )

  ;; Approach 2c: Multi-value (destructured params)
  (func (export "bench_multivalue_destr") (param $obj externref)
    (local $a externref) (local $b externref) (local $c externref)
    (call $destruct_mv3_destr (local.get $obj))
    (local.set $c) (local.set $b) (local.set $a)
    (table.set $refs (i32.const 0) (local.get $a))
    (table.set $refs (i32.const 1) (local.get $b))
    (table.set $refs (i32.const 2) (local.get $c))
  )

  ;; Approach 3: Per-field getters
  (func (export "bench_getters") (param $obj externref)
    (table.set $refs (i32.const 0) (call $get_a (local.get $obj)))
    (table.set $refs (i32.const 1) (call $get_b (local.get $obj)))
    (table.set $refs (i32.const 2) (call $get_c (local.get $obj)))
  )

  ;; Approach 4: Reflect.get per field (key strings built on JS side)
  (func (export "bench_reflect") (param $obj externref)
    (table.set $refs (i32.const 0) (call $reflect_get_a (local.get $obj)))
    (table.set $refs (i32.const 1) (call $reflect_get_b (local.get $obj)))
    (table.set $refs (i32.const 2) (call $reflect_get_c (local.get $obj)))
  )

  (func (export "get_result") (param $i i32) (result externref)
    (table.get $refs (local.get $i))
  )
)
