/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::{expect_panic, itest, next_frame};
use crate::object_tests::base_test::{Based, RefcBased};
use godot::classes::ClassDb;
use godot::prelude::*;
use godot::task::TaskHandle;

#[itest]
fn base_during_init() {
    let obj = Gd::<Based>::from_init_fn(|base| {
        // Test both temporary + local-variable syntax.
        base.to_init_gd().set_rotation(22.0);

        let mut gd = base.to_init_gd();
        gd.set_position(Vector2::new(100.0, 200.0));

        Based { base, i: 456 }
    });

    let guard = obj.bind();
    assert_eq!(guard.i, 456);
    assert_eq!(guard.base().get_rotation(), 22.0);
    assert_eq!(guard.base().get_position(), Vector2::new(100.0, 200.0));
    drop(guard);

    obj.free();
}

// This isn't recommended, but test what happens if someone clones and stores the Gd<T>.
#[itest]
fn base_during_init_extracted_gd() {
    let mut extractor = None;

    let obj = Gd::<Based>::from_init_fn(|base| {
        extractor = Some(base.to_init_gd());

        Based { base, i: 456 }
    });

    let extracted = extractor.expect("extraction failed");
    assert_eq!(extracted.instance_id(), obj.instance_id());
    assert_eq!(extracted, obj.clone().upcast());

    // Destroy through the extracted Gd<T>.
    extracted.free();
    assert!(
        !obj.is_instance_valid(),
        "object should be invalid after base ptr is freed"
    );
}

// Checks bad practice of rug-pulling the base pointer.
#[itest]
fn base_during_init_freed_gd() {
    let mut free_executed = false;

    expect_panic("base object is destroyed", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let obj = base.to_init_gd();
            obj.free(); // Causes the problem, but doesn't panic yet.
            free_executed = true;

            Based { base, i: 456 }
        });
    });

    assert!(
        free_executed,
        "free() itself doesn't panic, but following construction does"
    );
}

#[itest]
fn base_during_init_refcounted_simple() {
    {
        let obj = Gd::from_init_fn(|base| {
            eprintln!("---- before to_init_gd() ----");
            base.to_init_gd(); // Immediately dropped.
            eprintln!("---- after to_init_gd() ----");

            RefcBased { base }
        });
        eprintln!("After construction: refc={}", obj.get_reference_count());
    }

    // let mut last = Gd::<RefCounted>::from_instance_id(InstanceId::from_i64(-9223372001555511512));
    // last.call("unreference", &[]);
}

// Tests that the auto-decrement of surplus references also works when instantiated through the engine.
#[itest(async)]
fn base_during_init_refcounted_from_engine() -> TaskHandle {
    let db = ClassDb::singleton();
    let obj = db.instantiate("RefcBased").to::<Gd<RefcBased>>();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(async)]
fn base_during_init_refcounted_from_rust() -> TaskHandle {
    let obj = RefcBased::new_gd();

    assert_eq!(obj.get_reference_count(), 2);
    next_frame(move || assert_eq!(obj.get_reference_count(), 1, "eventual dec-ref happens"))
}

#[itest(focus)]
// #[itest]
fn base_during_init_refcounted_complex() {
    // Instantiate with multiple Gd<T> references.
    let (obj, base) = RefcBased::with_split();
    let id = obj.instance_id();
    dbg!(&id);
    dbg!(&obj);
    dbg!(id.to_i64() as u64);
    dbg!(base.instance_id().to_i64() as u64);

    // base.call("unreference", &[]);
    // base.call("unreference", &[]);

    assert_eq!(obj.instance_id(), base.instance_id());
    // assert_eq!(base.get_reference_count(), 2);
    // assert_eq!(obj.get_reference_count(), 2);

    drop(base);
    // assert_eq!(obj.get_reference_count(), 1);
    // assert_eq!(obj.get_reference_count(), 1);
    drop(obj);

    // assert!(!id.lookup_validity(), "last drop destroyed the object");
}

#[cfg(debug_assertions)]
#[itest]
fn base_during_init_outside_init() {
    let mut obj = Based::new_alloc();

    expect_panic("as_init_gd() outside init() function", || {
        let guard = obj.bind_mut();
        let _gd = guard.base.to_init_gd(); // Panics in Debug builds.
    });

    obj.free();
}

#[cfg(debug_assertions)]
#[itest]
fn base_during_init_to_gd() {
    expect_panic("WithBaseField::to_gd() inside init() function", || {
        let _obj = Gd::<Based>::from_init_fn(|base| {
            let temp_obj = Based { base, i: 999 };

            // This should panic because we're calling to_gd() during initialization
            let _gd = godot::obj::WithBaseField::to_gd(&temp_obj);

            temp_obj
        });
    });
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
