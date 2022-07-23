#![warn(clippy::all)]
#![warn(clippy::pedantic)]

//! Merge multiple [Juniper](https://docs.rs/juniper) object definitions into a single object type.
//!
//! [crates.io](https://crates.io/crates/juniper-compose) | [docs](https://docs.rs/juniper-compose) | [github](https://github.com/nikis05/juniper-compose)
//!
//! ## Motivation
//!
//! You are building a GraphQL server using Juniper. At some point you realize that you have gigantic
//! Query and Mutation types:
//!
//! ```
//! #[derive(Default)]
//! struct Query;
//!
//! #[juniper::graphql_object]
//! impl Query {
//!     async fn user(ctx: &Context, id: Uuid) -> User {
//!         // ...
//!     }
//!
//!     async fn users(ctx: &Context) -> Vec<User> {
//!         // ...
//!     }
//!
//!     async fn task(ctx: &Context, id: Uuid) -> Task {
//!         // ...
//!     }
//!
//!     async fn tasks(ctx: &Context) -> Vec<Task> {
//!         // ...
//!     }
//!     
//!     // ...many more
//! }
//! ```
//!
//! You would like to split it up into multiple domain-specific files, and have e.g. all User
//! queries in one file and all Task queries in the other. With current Juniper API, it is very
//! hard to do, but this crate can help you.
//!
//! ## Usage
//!
//! ```
//! #[derive(Default)]
//! struct UserQueries;
//!
//! #[composable_object]
//! #[juniper::graphql_object]
//! impl UserQueries {
//!     async fn user(ctx: &Context, id: Uuid) -> User {
//!         // ...
//!     }
//!
//!     async fn users(ctx: &Context) -> Vec<User> {
//!         // ...
//!     }
//! }
//!
//! #[derive(Default)]
//! struct TaskQueries;
//!
//! #[composable_object]
//! #[juniper::graphql_object]
//! impl TaskQueries {
//!     async fn task(ctx: &Context, id: Uuid) -> Task {
//!         // ...
//!     }
//!
//!     async fn tasks(ctx: &Context) -> Vec<Task> {
//!         // ...
//!     }
//! }
//!
//! composite_object!(Query(UserQueries, TaskQueries));
//! ```
//!
//! Custom contexts are supported:
//!
//! ```
//! composite_object!(Query<Context = MyCustomContext>(UserQueries, TaskQueries));
//! ```
//!
//! Custom scalars are currently not supported, but will be added if requested.

use juniper::{GraphQLTypeAsync, Type};
use std::borrow::Cow;

/// Implements [ComposableObject](ComposableObject) for a GraphQL object type.
/// **Important**: must be applied before the `juniper::graphql_object` macro.
///
/// ## Example
///
/// ```
/// #[composable_object]
/// #[graphql_object]
/// impl UserQueries {
///     // ...
/// }
/// ```
pub use juniper_compose_macros::composable_object;

/// Composes an object type from multiple [ComposableObject](ComposableObject)s.
/// Custom context type may be specified, otherwise defaults to `()`.
///
/// ## Examples
///
/// ```
/// composite_object!(Query(UserQueries, TaskQueries));
/// composite_object!(Mutation<Context = MyContextType>(UserMutations, TaskMutations));
/// ```
pub use juniper_compose_macros::composite_object;

/// Object types that you want to compose into one must implement this trait.
/// Use [composable_object](composable_object) to implement it.
pub trait ComposableObject: GraphQLTypeAsync + Default
where
    Self::Context: Sync,
    Self::TypeInfo: Sync,
{
    /// Returns a list of fields that exist on this object type.
    fn fields() -> &'static [&'static str];
}

#[doc(hidden)]
#[allow(clippy::must_use_candidate)]
pub fn type_to_owned<'a>(ty: &Type<'a>) -> Type<'static> {
    match ty {
        Type::Named(name) => Type::Named(Cow::Owned(name.to_string())),
        Type::NonNullNamed(name) => Type::NonNullNamed(Cow::Owned(name.to_string())),
        Type::List(inner) => Type::List(Box::new(type_to_owned(inner))),
        Type::NonNullList(inner) => Type::NonNullList(Box::new(type_to_owned(inner))),
    }
}
