# juniper-compose

Merge multiple [Juniper](https://docs.rs/juniper) object definitions into a single object type.

[crates.io](https://crates.io/crates/juniper-compose) | [docs](https://docs.rs/juniper-compose) | [github](https://github.com/nikis05/juniper-compose)

## Motivation

You are building a GraphQL server using Juniper. At some point you realize that you have gigantic
Query and Mutation types:

```rust
#[derive(Default)]
struct Query;

#[juniper::graphql_object]
impl Query {
    async fn user(ctx: &Context, id: Uuid) -> User {
        // ...
    }

    async fn users(ctx: &Context) -> Vec<User> {
        // ...
    }

    async fn task(ctx: &Context, id: Uuid) -> Task {
        // ...
    }

    async fn tasks(ctx: &Context) -> Vec<Task> {
        // ...
    }
    
    // ...many more
}
```

You would like to split it up into multiple domain-specific files, and have e.g. all User
queries in one file and all Task queries in the other. With current Juniper API, it is very
hard to do, but this crate can help you.

## Usage

```rust
#[derive(Default)]
struct UserQueries;

#[composable_object]
#[juniper::graphql_object]
impl UserQueries {
    async fn user(ctx: &Context, id: Uuid) -> User {
        // ...
    }

    async fn users(ctx: &Context) -> Vec<User> {
        // ...
    }
}

#[derive(Default)]
struct TaskQueries;

#[composable_object]
#[juniper::graphql_object]
impl TaskQueries {
    async fn task(ctx: &Context, id: Uuid) -> Task {
        // ...
    }

    async fn tasks(ctx: &Context) -> Vec<Task> {
        // ...
    }
}

composite_object!(Query(UserQueries, TaskQueries));
```

Custom contexts are supported:

```rust
composite_object!(Query<Context = MyCustomContext>(UserQueries, TaskQueries));
```

Visibility specifier for generated type is supported:

```rust
composite_object!(pub(crate) Query<Context = MyCustomContext>(UserQueries, TaskQueries));
```

Custom scalars are currently not supported, but will be added if requested.
