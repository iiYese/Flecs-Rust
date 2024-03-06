mod common;
use common::*;

fn iterate_components(entity: Entity) {
    // 1. The easiest way to print the components is to use get_archetype
    println!("[{}]", entity.get_archetype());
    println!();
    // 2. To get individual component ids, use for_each
    let mut count_components = 0;
    entity.for_each_component(|id| {
        println!("{}: {}", count_components, id.to_str());
        count_components += 1;
    });
    println!();

    // 3. we can also inspect and print the ids in our own way. This is a
    // bit more complicated as we need to handle the edge cases of what can be
    // encoded in an id, but provides the most flexibility.
    count_components = 0;

    entity.for_each_component(|id| {
        print!("{}: ", count_components);
        count_components += 1;
        if id.is_pair() {
            // If id is a pair, extract & print both parts of the pair
            let rel = id.first();
            let target = id.second();
            print!("rel: {}, target: {}", rel.get_name(), target.get_name());
        } else {
            // Id contains a regular entity. Strip role before printing.
            let comp = id.entity();
            print!("entity: {}", comp.get_name());
        }

        println!();
        println!();
    });
}
fn main() {
    let world = World::new();

    let bob = world
        .new_entity_named(CStr::from_bytes_with_nul(b"Bob\0").unwrap())
        .set(Position { x: 10.0, y: 20.0 })
        .set(Velocity { x: 1.0, y: 1.0 })
        .add::<Human>()
        .add_pair::<Eats, Apples>();

    println!("Bob's components:");
    iterate_components(bob);

    // We can use the same function to iterate the components of a component
    println!("Position's components:");
    iterate_components(world.component::<Position>().entity());

    // Output
    //  Bob's components:
    //  [Position, Velocity, Human, (Identifier,Name), (Eats,Apples)]
    //
    //  0: Position
    //  1: Velocity
    //  2: Human
    //  3: (Identifier,Name)
    //  4: (Eats,Apples)
    //
    //  0: entity: Position
    //  1: entity: Velocity
    //  2: entity: Human
    //  3: rel: Identifier, target: Name
    //  4: rel: Eats, target: Apples
    //
    //  Position's components:
    //  [Component, (Identifier,Name), (Identifier,Symbol), (OnDelete,Panic)]
    //
    //  0: Component
    //  1: (Identifier,Name)
    //  2: (Identifier,Symbol)
    //  3: (OnDelete,Panic)
    //
    //  0: entity: Component
    //  0: rel: Identifier, target: Name
    //  0: rel: Identifier, target: Symbol
    //  0: rel: OnDelete, target: Panic
}