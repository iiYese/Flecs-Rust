use std::ffi::c_char;
use std::ffi::CStr;

use flecs_ecs::core::*;
use flecs_ecs::sys;

pub trait IterOperations {
    #[doc(hidden)]
    fn retrieve_iter(&self) -> IterT;

    #[doc(hidden)]
    fn iter_next(&self, iter: &mut IterT) -> bool;

    #[doc(hidden)]
    fn iter_next_func(&self) -> unsafe extern "C" fn(*mut IterT) -> bool;

    #[doc(hidden)]
    fn query_ptr(&self) -> *const QueryT;
}

pub trait IterAPI<'a, P, T>: IterOperations + IntoWorld<'a>
where
    T: Iterable,
{
    // TODO once we have tests in place, I will split this functionality up into multiple functions, which should give a small performance boost
    // by caching if the query has used a "is_ref" operation.
    // is_ref is true for any query that contains fields that are not matched on the entity itself
    // so parents, prefabs but also singletons, or fields that are matched on a fixed entity (.with<Foo>().src_id(my_entity))
    /// Each iterator.
    /// The "each" iterator accepts a function that is invoked for each matching entity.
    /// The following function signatures is valid:
    ///  - func(comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Each iterators are automatically instanced.
    ///
    /// # See also
    ///
    /// * C++ API: `iterable::each`
    #[doc(alias = "iterable::each")]
    fn each(&self, mut func: impl FnMut(T::TupleType<'_>)) {
        unsafe {
            let mut iter = self.retrieve_iter();

            ecs_assert!(
                {
                    iter.flags |= sys::EcsIterCppEach;
                    true
                },
                "used to assert if using .field() in each functions."
            );

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = iter.count as usize;

                sys::ecs_table_lock(self.world_ptr_mut(), iter.table);

                for i in 0..iter_count {
                    let tuple = components_data.get_tuple(i);
                    func(tuple);
                }

                sys::ecs_table_unlock(self.world_ptr_mut(), iter.table);
            }
        }
    }

    /// Each iterator.
    /// The "each" iterator accepts a function that is invoked for each matching entity.
    /// The following function signatures is valid:
    ///  - func(e : Entity , comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Each iterators are automatically instanced.
    ///
    /// # See also
    ///
    /// * C++ API: `iterable::each`
    #[doc(alias = "iterable::each")]
    fn each_entity(&self, mut func: impl FnMut(EntityView, T::TupleType<'_>)) {
        unsafe {
            let mut iter = self.retrieve_iter();

            ecs_assert!(
                {
                    iter.flags |= sys::EcsIterCppEach;
                    true
                },
                "used to assert if using .field() in each functions."
            );

            let world = self.world_ptr_mut();
            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = {
                    if iter.count == 0 {
                        1_usize
                    } else {
                        iter.count as usize
                    }
                };

                sys::ecs_table_lock(world, iter.table);

                // TODO random thought, I think I can determine the elements is a ref or not before the for loop and then pass two arrays with the indices of the ref and non ref elements
                // I will come back to this in the future, my thoughts are somewhere else right now. If my assumption is correct, this will get rid of the branch in the for loop
                // and potentially allow for more conditions for vectorization to happen. This could potentially offer a (small) performance boost since the branch predictor avoids probably
                // most of the cost since the branch is almost always the same.
                // update: I believe it's not possible due to not knowing the order of the components in the tuple. I will leave this here for now, maybe I will come back to it in the future.
                for i in 0..iter_count {
                    let world = self.world();
                    let tuple = components_data.get_tuple(i);

                    func(EntityView::new_from(world, *iter.entities.add(i)), tuple);
                }

                sys::ecs_table_unlock(world, iter.table);
            }
        }
    }

    fn each_iter(&self, mut func: impl FnMut(&mut Iter<P>, usize, T::TupleType<'_>)) {
        unsafe {
            let mut iter = self.retrieve_iter();

            ecs_assert!(
                {
                    iter.flags |= sys::EcsIterCppEach;
                    true
                },
                "used to assert if using .field() in each functions."
            );

            let world = self.world_ptr_mut();

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = {
                    if iter.count == 0 {
                        1_usize
                    } else {
                        iter.count as usize
                    }
                };

                sys::ecs_table_lock(world, iter.table);

                let mut iter_t = Iter::new(&mut iter);

                for i in 0..iter_count {
                    let tuple = components_data.get_tuple(i);

                    func(&mut iter_t, i, tuple);
                }

                sys::ecs_table_unlock(world, iter.table);
            }
        }
    }

    /// find iterator to find an entity
    /// The "find" iterator accepts a function that is invoked for each matching entity and checks if the condition is true.
    /// if it is, it returns that entity.
    /// The following function signatures is valid:
    ///  - func(comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Each iterators are automatically instanced.
    ///
    /// # Returns
    ///
    /// * Some(EntityView<'a>) if the entity was found, None if no entity was found
    ///
    /// # See also
    ///
    /// * C++ API: `find_delegate::invoke_callback`
    #[doc(alias = "find_delegate::invoke_callback")]
    fn find(&self, mut func: impl FnMut(T::TupleType<'_>) -> bool) -> Option<EntityView<'a>> {
        unsafe {
            let mut iter = self.retrieve_iter();
            let mut entity: Option<EntityView> = None;
            let world = self.world_ptr_mut();

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = iter.count as usize;

                sys::ecs_table_lock(world, iter.table);

                for i in 0..iter_count {
                    let world = self.world();
                    let tuple = components_data.get_tuple(i);
                    if func(tuple) {
                        entity = Some(EntityView::new_from(world, *iter.entities.add(i)));
                        break;
                    }
                }

                sys::ecs_table_unlock(world, iter.table);
            }
            entity
        }
    }

    /// find iterator to find an entity
    /// The "find" iterator accepts a function that is invoked for each matching entity and checks if the condition is true.
    /// if it is, it returns that entity.
    /// The following function signatures is valid:
    ///  - func(entity : Entity, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Each iterators are automatically instanced.
    ///
    /// # Returns
    ///
    /// * Some(EntityView<'a>) if the entity was found, None if no entity was found
    ///
    /// # See also
    ///
    /// * C++ API: `find_delegate::invoke_callback`
    #[doc(alias = "find_delegate::invoke_callback")]
    fn find_entity(
        &self,
        mut func: impl FnMut(EntityView, T::TupleType<'_>) -> bool,
    ) -> Option<EntityView<'a>> {
        unsafe {
            let mut iter = self.retrieve_iter();
            let mut entity_result: Option<EntityView> = None;
            let world = self.world_ptr_mut();

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = iter.count as usize;

                sys::ecs_table_lock(world, iter.table);

                for i in 0..iter_count {
                    let world = self.world();
                    let entity = EntityView::new_from(world, *iter.entities.add(i));

                    let tuple = components_data.get_tuple(i);
                    if func(entity, tuple) {
                        entity_result = Some(entity);
                        break;
                    }
                }

                sys::ecs_table_unlock(world, iter.table);
            }
            entity_result
        }
    }

    /// find iterator to find an entity.
    /// The "find" iterator accepts a function that is invoked for each matching entity and checks if the condition is true.
    /// if it is, it returns that entity.
    /// The following function signatures is valid:
    ///  - func(iter : Iter, index : usize, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Each iterators are automatically instanced.
    ///
    /// # Returns
    ///
    /// * Some(EntityView<'a>) if the entity was found, None if no entity was found
    ///
    /// # See also
    ///
    /// * C++ API: `find_delegate::invoke_callback`
    #[doc(alias = "find_delegate::invoke_callback")]
    fn find_iter(
        &self,
        mut func: impl FnMut(&mut Iter<P>, usize, T::TupleType<'_>) -> bool,
    ) -> Option<EntityView<'a>> {
        unsafe {
            let mut iter = self.retrieve_iter();
            let mut entity_result: Option<EntityView> = None;
            let world = self.world_ptr_mut();

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = {
                    if iter.count == 0 {
                        1_usize
                    } else {
                        iter.count as usize
                    }
                };

                sys::ecs_table_lock(world, iter.table);
                let mut iter_t = Iter::new(&mut iter);

                for i in 0..iter_count {
                    let world = self.world();
                    let tuple = components_data.get_tuple(i);
                    if func(&mut iter_t, i, tuple) {
                        entity_result = Some(EntityView::new_from(world, *iter.entities.add(i)));
                        break;
                    }
                }

                sys::ecs_table_unlock(world, iter.table);
            }
            entity_result
        }
    }

    /// iter iterator.
    /// The "iter" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    ///  - func(it: &mut Iter, comp1 : &mut T1, comp2 : &mut T2, ...)
    ///
    /// Iter iterators are not automatically instanced. When a result contains
    /// shared components, entities of the result will be iterated one by one.
    /// This ensures that applications can't accidentally read out of bounds by
    /// accessing a shared component as an array.
    ///
    /// # See also
    ///
    /// * C++ API: `iterable::iter`
    #[doc(alias = "iterable::iter")]
    fn iter(&self, mut func: impl FnMut(&mut Iter<P>, T::TupleSliceType<'_>)) {
        unsafe {
            let mut iter = self.retrieve_iter();
            let world = self.world_ptr_mut();

            while self.iter_next(&mut iter) {
                let mut components_data = T::create_ptrs(&iter);
                let iter_count = iter.count as usize;

                sys::ecs_table_lock(world, iter.table);

                let tuple = components_data.get_slice(iter_count);
                let mut iter_t = Iter::new(&mut iter);
                func(&mut iter_t, tuple);

                sys::ecs_table_unlock(world, iter.table);
            }
        }
    }

    /// iter iterator.
    /// The "iter" iterator accepts a function that is invoked for each matching
    /// table. The following function signature is valid:
    ///  - func(it: &mut Iter)
    ///
    /// Iter iterators are not automatically instanced. When a result contains
    /// shared components, entities of the result will be iterated one by one.
    /// This ensures that applications can't accidentally read out of bounds by
    /// accessing a shared component as an array.
    ///
    /// # See also
    ///
    /// * C++ API: `iterable::iter`
    #[doc(alias = "iterable::iter")]
    fn iter_only(&self, mut func: impl FnMut(&mut Iter<P>)) {
        unsafe {
            let mut iter = self.retrieve_iter();
            let world = self.world_ptr_mut();
            while self.iter_next(&mut iter) {
                sys::ecs_table_lock(world, iter.table);
                let mut iter_t = Iter::new(&mut iter);
                func(&mut iter_t);
                sys::ecs_table_unlock(world, iter.table);
            }
        }
    }

    /// Get the entity of the current filter
    ///
    /// # Arguments
    ///
    /// * `filter`: the filter to get the entity from
    ///
    /// # Returns
    ///
    /// The entity of the current filter
    ///
    /// # See also
    ///
    /// * C++ API: `query_base::entity`
    #[doc(alias = "query_base::entity")]
    fn as_entity(&self) -> EntityView;

    /// Each term iterator.
    /// The `each_term` iterator accepts a function that is invoked for each term
    /// in the filter. The following function signature is valid:
    ///  - func(term: &mut Term)
    ///
    /// # See also
    ///
    /// * C++ API: `query_base::term`
    #[doc(alias = "query_base::each_term")]
    fn each_term(&self, mut func: impl FnMut(&TermRef)) {
        let query = self.query_ptr();
        ecs_assert!(
            !query.is_null(),
            FlecsErrorCode::InvalidParameter,
            "query filter is null"
        );
        let query = unsafe { &*query };
        for i in 0..query.term_count {
            let term = TermRef::new(&query.terms[i as usize]);
            func(&term);
        }
    }

    /// Get a immutable reference of the term of the current query at the given index
    /// This is mostly used for debugging purposes.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the term to get
    /// * `filter`: the filter to get the term from
    ///
    /// # Returns
    ///
    /// The term requested
    ///
    /// # See also
    ///
    /// * C++ API: `query_base::term`
    #[doc(alias = "query_base::term")]
    fn term(&self, index: usize) -> TermRef<'a> {
        let query = self.query_ptr();
        ecs_assert!(
            !query.is_null(),
            FlecsErrorCode::InvalidParameter,
            "query filter is null"
        );
        let query = unsafe { &*query };
        TermRef::new(&query.terms[index])
    }

    /// Get the field count of the current filter
    ///
    /// # Arguments
    ///
    /// * `filter`: the filter to get the field count from
    ///
    /// # Returns
    ///
    /// The field count of the current filter
    ///
    /// # See also
    ///
    /// * C++ API: `query_base::field_count`
    #[doc(alias = "query_base::field_count")]
    fn field_count(&self) -> i8 {
        let query = self.query_ptr();
        unsafe { (*query).field_count }
    }

    /// Get the count of terms set of the current query
    fn term_count(&self) -> u32 {
        let query = self.query_ptr();
        unsafe { (*query).term_count as u32 }
    }

    /// Convert filter to string expression. Convert filter terms to a string expression.
    /// The resulting expression can be parsed to create the same filter.
    ///
    /// # Arguments
    ///
    /// * `filter`: the filter to convert to a string
    ///
    /// # Returns
    ///
    /// The string representation of the filter
    ///
    /// # See also
    ///
    /// * C++ API: `query_base::str`
    #[doc(alias = "query_base::str")]
    #[allow(clippy::inherent_to_string)] // this is a wrapper around a c function
    fn to_string(&self) -> String {
        let query = self.query_ptr();
        let result: *mut c_char = unsafe { sys::ecs_query_str(query as *const _) };
        let rust_string =
            String::from(unsafe { std::ffi::CStr::from_ptr(result).to_str().unwrap() });
        unsafe {
            if let Some(free_func) = sys::ecs_os_api.free_ {
                free_func(result as *mut _);
            }
        }
        rust_string
    }

    fn find_var(&self, name: &CStr) -> Option<i32> {
        let var_index = unsafe { sys::ecs_query_find_var(self.query_ptr(), name.as_ptr()) };
        if var_index == -1 {
            None
        } else {
            Some(var_index)
        }
    }

    fn plan(&self) -> String {
        let query = self.query_ptr();
        let result: *mut c_char = unsafe { sys::ecs_query_plan(query as *const _) };
        let rust_string =
            String::from(unsafe { std::ffi::CStr::from_ptr(result).to_str().unwrap() });
        unsafe {
            if let Some(free_func) = sys::ecs_os_api.free_ {
                free_func(result as *mut _);
            }
        }
        rust_string
    }

    fn iterable(&self) -> IterIterable<P, T> {
        IterIterable::new(self.retrieve_iter(), self.iter_next_func())
    }

    /// Return first matching entity.
    ///
    /// # See also
    ///
    /// * C++ API: `iterable::first`
    /// * C++ API: `iter_iterable::first`
    #[doc(alias = "iterable::first")]
    #[doc(alias = "iter_iterable::first")]
    fn first_entity(&mut self) -> Option<EntityView<'a>> {
        let mut entity = None;

        let it = &mut self.retrieve_iter();

        if self.iter_next(it) && it.count > 0 {
            entity = Some(EntityView::new_from(self.world(), unsafe {
                *it.entities.add(0)
            }));
            unsafe { sys::ecs_iter_fini(it) };
        }
        entity
    }

    /// Returns true if iterator yields at least once result.
    fn is_true(&mut self) -> bool {
        let mut it = self.retrieve_iter();

        let result = self.iter_next(&mut it);
        if result {
            unsafe { sys::ecs_iter_fini(&mut it) };
        }
        result
    }

    /// Return total number of entities in result.
    ///
    /// # Returns
    ///
    /// The total number of entities in the result
    ///
    /// # See also
    ///
    /// * C++ API: `iter_iterable::count`
    #[doc(alias = "iter_iterable::count")]
    fn count(&mut self) -> i32 {
        let mut it = self.retrieve_iter();
        let mut result = 0;
        while self.iter_next(&mut it) {
            result += it.count;
        }
        result
    }
}
