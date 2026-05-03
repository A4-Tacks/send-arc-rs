`Arc` wrapper, but sending `Arc` requires `Send + Sync`, while sending `SendArc` only requires `Sync`

- `Arc<T>` because `T` needs to be drop in other threads, so sending `Arc` requires `T: Send + Sync`
- `SendArc<T>` is only drop on the original thread, so sending `SendArc` requires `T: Sync`, this is consistent with the reference

# What's the usage?
Used like `&'a T`, but without `'a`, guaranteed by runtime

If some APIs do not have something like `std::thread::scope`, you can use `SendArc`
