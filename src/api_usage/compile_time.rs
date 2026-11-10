
#[partly(dynamic)]
// The macro should create a FooPartial and a FooPatch. We need attributes so you an disable the creation of either one
// struct where al fields are the same, just as Option<>
// when using [Partly()] is will create both the dynamic and the compile-time partials
struct Foo {
    bar: u32,
    zee: String,

    // we need a way so certain fields an be excluded from partly
}

fn template_example<T: FooPartial + FooHasBar>(T: &input) {
    info!("{}", input.bar());
}

fn example_partial() {
    let foo_value = Foo { bar: 0, zee: "yay".into()};

    // Same as FooPartial::new();
    let partial: FooPartial<()> = Foo::partial();

    let partial: FooPartial<(BarValue,)> = partial.set_bar(12);

    if partial.has_zee() { // compile time evaluated
        print!("Partial has a zee value") 
    }

    let bar_value = partial.bar(); // Ok, bar has been set
    let zee_value = partial.zee(); // Comnpile error, we have not set zee yet.

    template_example(&partial);

    let b: bool = partial.is_complete(); // Returns true if all values are set

    // can_build and build are actualy their own trait
    // We need a way for the derive to skip implementing them, so the user can
    // build them manually
    if partial.can_build() { // false if partial is not complete.
        let foo: Foo = partial.build?; // Compile error, if partial is not complete
        let partial = foo.into_partial(); // Will set all the fields
    }

    // see dynamic.rs on its usage. Only if the partial derive has actually created dynamic partials for foo.
    let dynamic_partial = partial.into_dynamic();

    let empty: bool = partial.is_empty(); //True only, when no values have been set. Compile time evaluation

    foo_value.set(partial); //Will set all the values of foo_value are are Some on the corresponding 'partial'

      
}

struct PatchValue<T> {
    old_value: T,
    new_value: T,
}

// A patch is similar to a partial. For the above Foo, the patch would look something like:
struct Foo {
    bar: Option<PatchValue<u32>>,
    zee: Option<PatchValue<String>>,
}


fn example_patch() {
    let foo_value = Foo { bar: 0, zee: "yay".into()};

    let foo_patch = BoundFooPatch::new(&foo_value); //whenever we set a value in the future, we will use the corresponding foo_value as the old value.

    let foo_patch = foo_patch.set_bar(15); //Now foo_patch stores 0 as the old value for bar and 15 as the new one.

    foo_value.apply_patch(foo_patch.clone())?; // works just fine
    foo_patch.bar(17); //does not create a new type, because bar is already set. So it's the same type, just the value gets overwritten.

    foo_value.bar(32); 
    foo_value.apply_patch(foo_patch)?; //Will fail at runtime, because old foo_value's bar value has changed so the patch no longer applies validly.

    let mut foo_patch2 = UnboundFooPatch::new(); //Whenever we set a value, we have to provide the old value ourselves
    foo_patch2.zee(PatchValue {old_value: String::new(), new_value: String::from("something cool")});

    let foo_patch = foo_patch.merge(foo_patch2); // Partials and patches can be merged. If poth patches contain different fields, this will yield a new type.

    let empty: bool = foo_patch.empty(); //True, when no fields have been set. Compile time evaluated
    let complete: bool = foo_patch.complete(); // True, when all fields have been set. Compile time evaluated

}
