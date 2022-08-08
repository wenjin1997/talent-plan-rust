# Notes -- Building Blocks 4

## **[Reading: Fearless Concurrency with Rust](https://blog.rust-lang.org/2015/04/10/Fearless-Concurrency.html)**

Here's a taste of concurrency in Rust:

- A [channel](https://static.rust-lang.org/doc/master/std/sync/mpsc/index.html) transfers ownership of the messages sent along it, so you can send a pointer from one thread to another without fear of the threads later racing for access through that pointer. **Rust's channels enforce thread isolation.**
- A [lock](https://static.rust-lang.org/doc/master/std/sync/struct.Mutex.html) knows what data it protects, and Rust guarantees that the data can only be accessed when the lock is held. State is never accidentally shared. **"Lock data, not code" is enforced in Rust.**
- Every data type knows whether it can safely be [sent](https://static.rust-lang.org/doc/master/std/marker/trait.Send.html) between or [accessed](https://static.rust-lang.org/doc/master/std/marker/trait.Sync.html) by multiple threads, and Rust enforces this safe usage; there are no data races, even for lock-free data structures. **Thread safety isn't just documentation; it's law.**
- You can even [share stack frames](https://doc.rust-lang.org/1.0.0/std/thread/fn.scoped.html) between threads, and Rust will statically ensure that the frames remain active while other threads are using them. **Even the most daring forms of sharing are guaranteed safe in Rust**.

### Background: borrowing

Each reference is valid for a limited scope, which the compiler will automatically determine. References come in two flavors:

- Immutable references `&T`, which allow sharing but not mutation. There can be multiple `&T` references to the same value simultaneously, but the value cannot be mutated while those references are active.
- Mutable references `&mut T`, which allow mutation but not sharing. If there is an `&mut T` reference to a value, there can be no other active references at that time, but the value can be mutated.

Rust checks these rules at compile time; borrowing has no runtime overhead.

Why have two kinds of references? Consider a function like:

```rust
fn push_all(from: &Vec<i32>, to: &mut Vec<i32>) {
    for i in from.iter() {
        to.push(*i);
    }
}
```

This function iterates over each element of one vector, pushing it onto another. The iterator keeps a pointer into the vector at the current and final positions, stepping one toward the other.

What if we called this function with the same vector for both arguments?

```rust
push_all(&vec, &mut vec)
```

This would spell disaster! As we're pushing elements onto the vector, it will occasionally need to resize, allocating a new hunk of memory and copying its elements over to it. The iterator would be left with a dangling pointer into the old memory, leading to memory unsafety (with attendant segfaults or worse).

Fortunately, Rust ensures that **whenever a mutable borrow is active, no other borrows of the object are active**, producing the message:

```bash
error: cannot borrow `vec` as mutable because it is also borrowed as immutable
push_all(&vec, &mut vec);
                    ^~~
```

### Message passing

**Rust's ownership makes it easy to turn that advice into a compiler-checked rule**. Consider the following channel API ([channels in Rust's standard library](https://static.rust-lang.org/doc/master/std/sync/mpsc/index.html) are a bit different):

```rust
fn send<T: Send>(chan: &Channel<T>, t: T);
fn recv<T: Send>(chan: &Channel<T>) -> T;
```

Channels are generic over the type of data they transmit (the `<T: Send>` part of the API). The `Send` part means that `T` must be considered safe to send between threads; we'll come back to that later in the post, but for now it's enough to know that `Vec<i32>` is `Send`.

As always in Rust, passing in a `T` to the `send` function means transferring ownership of it. This fact has profound consequences: it means that code like the following will generate a compiler error.

```rust
// Suppose chan: Channel<Vec<i32>>

let mut vec = Vec::new();
// do some computation
send(&chan, vec); // vec 将所有权传给了 send 函数
print_vec(&vec); 	// 这里再使用 vec 是错误的
```

Here, the thread creates a vector, sends it to another thread, and then continues using it. The thread receiving the vector could mutate it as this thread continues running, so the call to `print_vec` could lead to race condition or, for that matter, a use-after-free bug.

Instead, the Rust compiler will produce an error message on the call to `print_vec`:

```
Error: use of moved value `vec`
```

### Locks

In Rust, threads are "isolated" from each other automatically, due to ownership. Writes can only happen when the thread has mutable access, either by owning the data, or by having a mutable borrow of it. Either way, **the thread is guaranteed to be the only one with access at the time**. 

Here is a simplified version (the [standard library's](https://static.rust-lang.org/doc/master/std/sync/struct.Mutex.html) is more ergonomic):

```rust
// create a new mutex
fn mutex<T: Send>(t: T) -> Mutex<T>;

// acquire the lock
fn lock<T: Send>(mutex: &Mutex<T>) -> MutexGuard<T>;

// access the data protected by the lock
fn access<T: Send>(guard: &mut MutexGuard<T>) -> &mut T;
```

This lock API is unusual in several respects.

First, the `Mutex` type is generic over a type `T` of **the data protected by the lock**. When you create a `Mutex`, you transfer ownership of that data *into* the mutex, immediately giving up access to it. (Locks are unlocked when they are first created.)

Later, you can `lock` to block the thread until the lock is acquired. This function, too, is unusual in providing a return value, `MutexGuard<T>`. The `MutexGuard` automatically releases the lock when it is destroyed; there is no separate `unlock` function.

The only way to access the lock is through the `access` function, which turns a mutable borrow of the guard into a mutable borrow of the data (with a shorter lease):

```rust
fn use_lock(mutex: &Mutex<Vec<i32>>) {
    // acquire the lock, taking ownership of a guard;
    // the lock is held for the rest of the scope
    let mut guard = lock(mutex);

    // access the data by mutably borrowing the guard
    let vec = access(&mut guard);

    // vec has type `&mut Vec<i32>`
    vec.push(3);

    // lock automatically released here, when `guard` is destroyed
}
```

There are two key ingredients here:

- The mutable reference returned by `access` cannot outlive the `MutexGuard` it is borrowing from. 通过 `access` 函数返回的可变引用不能比它借用的 `MutexGuard` 活得更久。
- The lock is only released when the `MutexGuard` is destroyed. 只有当 `MutexGuard` 被销毁时 lock 才释放。

The result is that **Rust enforces locking discipline: it will not let you access lock-protected data except when holding the lock**. 

### Thread safety and "Send"

For example, Rust ships with two kinds of "smart pointers" for reference counting:

- `Rc<T>` provides reference counting via normal reads/writes. It is not thread safe.
- `Arc<T>` provides reference counting via *atomic* operations. It is thread safe.

在Rust中数据类型被分为两类，`Send` 表示可以安全的在线程之间 move ，另一种是 `!Send` ，意味着这样做可能不安全。自然地，`Arc`是`Send` ，而 `Rc` 不是。

```
`Rc<Vec<i32>>` cannot be sent between threads safely
```

### Sharing the stack: "scoped"

But what if we wanted to start some threads that make use of data living in our stack frame? That could be dangerous:

```rust
fn parent() {
    let mut vec = Vec::new();
    // fill the vector
    thread::spawn(|| {
        print_vec(&vec)
    })
}
```

子线程获取对 `vec`  的引用，而父线程的堆栈中还保留有 `vec` 。当父线程退出时，堆栈帧被弹出，但子线程却没有。Oops!

为了排除这种内存不安全，Rust 基本线程线程生成API看起来有点像这样：

```rust
fn spawn<F>(f: F) where F: 'static, ...
```

`'static` 加有限制，在闭包中不能有借用的数据。这也意味着上面的 `parent` 函数会产生错误：

```
error: `vec` does not live long enough
```

但是还有另一种保证安全性的方法：确保父堆栈帧保持不变，直到子线程完成。这是 fork-join 变成的模式，通常用于分治并行算法。Rust 通过提供线程生成的 "scoped" variant 来支持它：

```rust
fn scoped<'a, F>(f: F) -> JoinGuard<'a> where F: 'a, ...
```

与上面的 `spawn` API 有两个主要区别：

* 使用参数 `'a` ，而不是 `'static` 。此参数表示一个范围，该范围包含闭包 `f` 中的所有借用。
* 返回值时一个 `JoinGuard` 。顾名思义， `JoinGuard` 通过在其析构函数中执行隐式连接（如果尚未显式发生），确保父线程加入（等待）其子线程。

在 `JoinGuard` 中包含 `'a` 可确保 `JoinGuard` 无法逃脱闭包借用的任何数据的范围。 换句话说，Rust 保证父线程在弹出子线程可能访问的任何堆栈帧之前等待子线程完成。

Thus by adjusting our previous example, we can fix the bug and satisfy the compiler:

```rust
fn parent() {
    let mut vec = Vec::new();
    // fill the vector
    let guard = thread::scoped(|| {
        print_vec(&vec)
    });
    // guard destroyed here, implicitly joining
}
```

So in Rust, you can freely borrow stack data into child threads, confident that the compiler will check for sufficient synchronization.

### Data races

Rust 并发方法：编译器防止所有数据竞争。

> *A data race is any unsynchronized, concurrent access to data involving a write.*

Synchronization here includes things as low-level as atomic instructions. Essentially, this is a way of saying that you cannot accidentally "share state" between threads; all (mutating) access to state has to be mediated by *some* form of synchronization.

Data races are just one (very important) kind of race condition, but by preventing them, Rust often helps you prevent other, more subtle races as well. For example, it's often important that updates to different locations appear to take place *atomically*: other threads see either all of the updates, or none of them. In Rust, having `&mut` access to the relevant locations at the same time **guarantees atomicity of updates to them**, since no other thread could possibly have concurrent read access.

It's worth pausing for a moment to think about this guarantee in the broader landscape of languages. Many languages provide memory safety through garbage collection. But garbage collection doesn't give you any help in preventing data races.

Rust instead uses ownership and borrowing to provide its two key value propositions:

- Memory safety without garbage collection.
- Concurrency without data races.

## **[Reading: What is the difference between concurrency and parallelism?](https://stackoverflow.com/questions/1050222/what-is-the-difference-between-concurrency-and-parallelism#1050257)**

> **Concurrency** is when two or more tasks can start, run, and complete in overlapping time **periods**. It doesn't necessarily mean they'll ever both be running **at the same instant**. For example, *multitasking* on a single-core machine.
>
> **Parallelism** is when tasks *literally* run at the same time, e.g., on a multicore processor.

并发和并行的区别，并发看作是一段时间内的，多个线程交替执行。并行是在一个时间点，真的有多个任务在同一时刻执行。

## **[Reading: Rust: A unique perspective](https://limpet.net/mbrubeck/2019/02/07/rust-a-unique-perspective.html)**

### Unique access

The first key observation is: **If a variable has unique access to a value, then it is safe to mutate it.**

安全，我的意思是内存安全：没有无效的指针访问、数据竞争或其他未定义行为的原因。 并且通过唯一访问，我的意思是当这个变量是活动的时，没有其他变量可以用来读取或写入相同值的任何部分。
唯一访问使内存安全变得非常简单：如果没有其他指向该值的指针，那么您不必担心使它们无效。 同样，如果其他线程上的变量无法访问该值，则无需担心同步。

#### Unique ownership

One form of unique access is **ownership**. When you initialize a variable with a value, it becomes the sole *owner* of that value. Because the value has just one owner, the owner can safely mutate the value, destroy it, or transfer it to a new owner.

Depending on the type of the value, assigning a value to a new variable will either **move** it or **copy** it. Either way, unique ownership is preserved. For a *move* type, the old owner becomes inaccessible after the move, so we still have one value owned by one variable:

```rust
let x = vec![1, 2, 3];
let y = x;             // move ownership from x to y
// can’t access x after moving its value to y
```

For a *copy* type, the value is duplicated, so we end up with two values owned by two variables:

```rust
let x = 1;
let y = x; // copy the value of x into y
```

In this case, each variable ends up with a separate, independent value. Mutating one will not affect the other.

One value might be owned by another value, rather than directly by a variable. For example, a struct owns its fields, a `Vec<T>` owns the `T` items inside it, and a `Box<T>` owns the `T` that it points to.

#### Unique borrowing

If you have unique access to a value of type `T`, you can borrow a **unique reference** to that value. A unique reference to a `T` has type `&mut T`.

Because it’s safe to mutate when you have a unique reference, unique references are also called “mutable references.“

The Rust compiler enforces this uniqueness at compile time. In any region of code where the unique reference may be used, no other reference to any part of the same value may exist, and even the owner of that value may not move or destroy it. Violating this rule triggers a compiler error.

A reference only **borrows** the value, and must return it to its owner. This means that the reference can be used to mutate the value, but not to move or destroy it (unless it overwrites it with a new value, for example using [`replace`](https://doc.rust-lang.org/std/mem/fn.replace.html)). Just like in real life, you need to give back what you’ve borrowed.

Borrowing a value is like locking it. Just like a mutex lock in a multi-threaded program, it’s usually best to hold a borrowed reference for as little time as possible. Storing a unique reference in a long-lived data structure will prevent any other use of the value for as long as that structure exists.

#### Unique references can't be copied

An `&mut T` cannot be copied or cloned, because this would result in two ”unique” references to the same value. It can only be moved:

```rust
let mut a = 1;
let x = &mut a;
let y = x; // move the reference from x into y
// x is no longer accessible here
```

However, you can temporarily ”re-borrow” from a unique reference. This gives a new unique reference to the same value, but the original reference can no longer be accessed until the new one goes out of scope or is no longer used (depending on which version of Rust you are using):

```rust
let mut a = 1;
let x = &mut a;
{
    let y = &mut *x;
    // x is "re-borrowed" and cannot be used while y is alive
    *y = 4; // y has unique access and can mutate `a`
}
// x becomes accessible again after y is dead
*x += 1; // now x has unique access again and can mutate the value
assert_eq!(*x, 5);
```

Re-borrowing happens implicitly when you call a function that takes a unique reference. This greatly simplifies code that passes unique references around, but can confuse programmers who are just learning about these restrictions.

### Shared access

A value is **shared** if there are multiple variables that are alive at the same time that can be used to access it.

While a value is shared, we have to be a lot more careful about mutating it. Writing to the value through one variable could invalidate pointers held by other variables, or cause a data race with readers or writers on other threads.

Rust ensures that **you can read from a value only while no variables can write to it**, and **you can write to a value only while no other variables can read or write to it.** In other words, you can have a unique writer, *or* multiple readers, but not both at once. Some Rust types enforce this at compile time and others at run time, but the principle is always the same.

#### Shared ownership

One way to share a value of type `T` is to create an `Rc<T>`, or “reference-counted pointer to T”. This allocates space on the heap for a `T`, plus some extra space for reference counting (tracking the number of pointers to the value). Then you can call `Rc::clone` to increment the reference count and receive another `Rc<T>` that points to the same value:

```rust
let x = Rc::new(1);
let y = x.clone();
// x and y hold two different Rc that point to the same memory
```

Because the `T` lives on the heap and `x` and `y` just hold pointers to it, it can outlive any particular pointer. It will be destroyed only when the last of the pointers is dropped. This is called **shared ownership**.

#### Shared borrowing

Since `Rc<T>` doesn't have unique access to its `T`, it can’t give out a unique `&mut T` reference (unless it checks at run time that the reference count is equal to 1, so it is not actually shared). But it *can* give out a **shared reference to T**, whose type is written `&T`. (This is also called an “immutable reference.”)

A shared reference is another “borrowed” type which can’t outlive its referent. The compiler ensures that a shared reference can’t be created while a unique reference exists to any part of the same value, and vice-versa. **And (just like unique references) the owner isn’t allowed to drop/move/mutate the value while any shared references are alive.**

If you have unique access to a value, you can produce many shared references or one unique reference to it. However, if you only have shared access to a value, you can’t produce a unique reference (at least, not without some additional checks, which I’ll discuss soon). One consequence of this is that you can convert an `&mut T` to an `&T`, but not vice-versa.

Because multiple shared references are allowed, an `&T` can be copied/cloned (unlike `&mut T`).

#### Thread safety

Astute readers might notice that merely cloning an `Rc<T>` mutates a value in memory, since it modifies the reference count. This could cause a data race if another clone of the `Rc` were accessed at the same time on a different thread! The compiler solves this in typical Rust fashion: By refusing to compile any program that passes an `Rc` to a different thread.

Rust has two built-in traits that it uses to mark types that can be accessed safely by other threads:

- **`T: Send`** means it's safe to access a `T` on a single other thread, where one thread at a time has exclusive access. A value of this type can be moved to another thread by unique ownership, or borrowed on another thread by unique reference (`&mut T`). A more descriptive name for this trait might be **`UniqueThreadSafe`**.
- **`T: Sync`** means it’s safe for many threads to access a `T` simultaneously, with each thread having shared access. Values of such types can be accessed on other threads via shared ownership or shared references (`&T`). A more descriptive name would be **`SharedThreadSafe`**.

`Rc<T>` implements neither of these traits, so an `Rc<T>` cannot be moved or borrowed into a variable on a different thread. It is forever trapped on the thread where it was born.

The standard library also offers an `Arc<T>` type, which is exactly like `Rc<T>` except that it implements `Send`, and uses atomic operations to synchronize access to its reference counts. This can make `Arc<T>` a little more expensive at run time, but it allows multiple threads to share a value safely.

These traits are not mutually exclusive. Many types are both `Send` and `Sync`, meaning that it’s safe to give unique access to one other thread (for example, moving the value itself or sending an `&mut T` reference) *or* shared access to many threads (for example, sending multiple `Arc<T>` or `&T`).

![image-20220806095747669](/rust/building-blocks/img/image-20220806095747669.png)

#### Shared mutability

So far, we’ve seen that sharing is safe when values are not mutated, and mutation is safe when values are not shared. But what if we want to share *and* mutate a value? The Rust standard library provides several different mechanisms for **shared mutability**.

The official documentation also calls this “interior mutability” because it lets you mutate a value that is “inside” of an immutable value. This terminology can be confusing: What does it mean for the exterior to be “immutable” if its interior is mutable? I prefer “shared mutability” which puts the spotlight on a different question: How can you safely mutate a value while it is shared?

##### What could go wrong?

What’s the big deal about shared mutation? Let’s start by listing some of the ways it could go wrong:

First, mutating a value can cause **pointer invalidation**. For example, pushing to a vector might cause it to reallocate its buffer. If there are other variables that contained addresses of items in the buffer, they would now point to deallocated memory. Or, mutating an enum might overwrite a value of one type with a value of a different type. A pointer to the old value will now be pointing at memory occupied by the wrong type. Either of these cases would trigger undefined behavior.

Second, it could violate **aliasing assumptions**. For example, the optimizing compiler assumes by default that the referent of an `&T` reference will not change while the reference exists. It might re-order code based on this assumption, leading to undefined behavior when the assumption is violated.

Third, if one thread mutates a value at the same time that another thread is accessing it, this causes a **data race** unless both threads use [synchronization](https://doc.rust-lang.org/std/sync/) primitives to prevent their operations from overlapping. Data races can cause arbitrary undefined behavior (in part because data races can also violate assumptions made by the optimizer during code generation).

##### UnsafeCell

To fix the problem of aliasing assumptions, we need [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html). The compiler knows about this type and treats it specially: It tells the optimizer that the value inside an `UnsafeCell` is not subject to the usual restrictions on aliasing.

Safe Rust code doesn’t use `UnsafeCell` directly. Instead, it’s used by libraries (including the standard library) that provide APIs for *safe* shared mutability. All of the shared mutable types discussed in the following sections use `UnsafeCell` internally.

`UnsafeCell` solves only one of the three problems listed above. Next, we'll see some ways to solve the other two problems: pointer invalidation and data races.

##### Multi-threaded shared mutability

Rust programs can safely mutate a value that’s shared across threads, as long as the basic rules of unique and shared access are enforced: Only one thread at a time may have unique access to a value, and only this thread can mutate it. When no thread has unique access, then many threads may have shared access, but the value can’t be mutated while they do.

Rust has two main types that allow thread-safe shared mutation:

- **`Mutex<T>`** allows one thread at a time to “lock” a mutex and get unique access to its contents. If a second thread tries to lock the mutex at the same time, the second thread will block until the first thread unlocks it. Since `Mutex` provides access to only one thread at a time, it can be used to share any type that implements the `Send` (“unique thread-safe”) trait.
- **`RwLock<T>`** is similar but has two different types of lock: A “write” lock that provides unique access, and a “read” lock that provides shared access. It will allow many threads to hold read locks at the same time, but only one thread can hold a write lock. If one thread tries to write while other threads are reading (or vice-versa), it will block until the other threads release their locks. Since `RwLock` provides both unique and shared access, its contents must implement both `Send` (“unique thread-safe”) and `Sync` (“shared thread-safe”).

These types prevent pointer invalidation by using run-time checks to enforce the rules of unique and shared borrowing. They prevent data races by using synchronization primitives provided by the platform’s native threading system.

In addition, various **[atomic types](https://doc.rust-lang.org/std/sync/atomic/)** allow safe shared mutation of individual primitive values. These prevent data races by using compiler intrinsics that provide synchronized operations, and they prevent pointer invalidation by refusing to give out references to their contents; you can only read from them or write to them by value.

All these types are only useful when shared by multiple threads, so they are often used in combination with `Arc`. Because `Arc` lets multiple threads share ownership of a value, it works with threads that might outlive the function that spawns them (and therefore can’t borrow references from it). However, [scoped threads](https://docs.rs/crossbeam/0.3.2/crossbeam/struct.Scope.html#method.spawn) are guaranteed to terminate before their spawning function, so they can capture shared references like `&Mutex<T>` instead of `Arc<Mutex<T>>`.

##### Single-threaded shared mutability

The standard library also has two types that allow safe shared mutation within a single thread. These types don’t implement the `Sync` trait, so the compiler won't let you share them across multiple threads. This neatly avoids data races, and also means that these types don’t need atomic operations (which are potentially expensive).

- **`Cell<T>`** solves the problem of pointer invalidation by forbidding pointers to its contents. Like the atomic types mentioned above, you can only read from it or write to it by value. Changing the data “inside” of the `Cell<T>` is okay, because there are no shared pointers to that data – only to the `Cell<T>` itself, whose type and address do not change when you mutate its interior. (Now we see why “interior mutability” is also a useful concept.)
- Many Rust types are useless without references, so Cell is often too restrictive. **`RefCell<T>`** allows you to borrow either unique or shared references to its contents, but it keeps count of how many borrowers are alive at a time. Like `RwLock`, it allows one unique reference or many shared references, but not both at once. It enforces this rule using run-time checks. (But since it’s used within a single thread, it can’t block the thread while waiting for other borrowers to finish. Instead, it panics if a program violates its borrowing rules.)

These types are often used in combination with `Rc<T>`, so that a value shared by multiple owners can still be mutated safely. They may also be used for mutating values behind shared references. The [`std::cell`](https://doc.rust-lang.org/std/cell/) docs have some examples.

### Summary

To summarize some key ideas:

- Rust has two types of references: unique and shared.
- Unique mutable access is easy.
- Shared immutable access is easy.
- Shared mutable access is hard.
- This is true for both single-threaded and multi-threaded programs.

We also saw a couple of ways to classify Rust types. Here’s a table showing some of the most common types according to this classification scheme:

|          | Unique      | Shared            |
| -------- | ----------- | ----------------- |
| Borrowed | `&mut T`    | `&T`              |
| Owned    | `T, Box<T>` | `Rc<T>`, `Arc<T>` |

I hope that thinking of these types in terms of uniqueness and sharing will help you understand how and why they work, as it helped me.

### Want to know more?

As I said at the start, this is just a quick introduction and glosses over many details. The exact rules about unique and shared access in Rust are still being worked out. The [Aliasing](https://doc.rust-lang.org/nomicon/aliasing.html) chapter of the Rustonomicon explains more, and Ralf Jung’s [Stacked Borrows](https://www.ralfj.de/blog/2018/11/16/stacked-borrows-implementation.html) model is the start of a more complete and formal definition of the rules.

If you want to know more about how shared mutability can lead to memory-unsafety, read [The Problem With Single-threaded Shared Mutability](https://manishearth.github.io/blog/2015/05/17/the-problem-with-shared-mutability/) by Manish Goregaokar.

The Swift language has an approach to memory safety that is similar in some ways, though its exact mechanisms are different. You might be interested in its recently-introduced [Exclusivity Enforcement](https://swift.org/blog/swift-5-exclusivity/) feature, and the [Ownership Manifesto](https://github.com/apple/swift/blob/fa952d398611e9a2b97531e2ac3efb6c36e9ba98/docs/OwnershipManifesto.md) that originally described its design and rationale.

## **[Video: Rust Concurrency Explained](https://www.youtube.com/watch?v=Dbytx0ivH7Q)**

Safety in Rust

* Rust statically prevents  aliasing + mutation
* Ownership prevents double-free
* Borrowing prevents use-after-free
* Overall, no segfaults!

Date races: Sharing + Mutation + No ordering

## **[Reading: `std::sync`](https://doc.rust-lang.org/std/sync/index.html)**

### [The need for synchronization](https://doc.rust-lang.org/std/sync/index.html#the-need-for-synchronization)

```rust
static mut A: u32 = 0;
static mut B: u32 = 0;
static mut C: u32 = 0;

fn main() {
    unsafe {
        A = 3;
        B = 4;
        A = A + B;
        C = B;
        println!("{A} {B} {C}");
        C = A;
    }
}
```

Note that thanks to Rust’s safety guarantees, accessing global (static) variables requires `unsafe` code, assuming we don’t use any of the synchronization primitives in this module.

### [Out-of-order execution](https://doc.rust-lang.org/std/sync/index.html#out-of-order-execution)

Instructions can execute in a different order from the one we define, due to various reasons:

- The **compiler** reordering instructions.
- A **single processor** executing instructions [out-of-order](https://en.wikipedia.org/wiki/Out-of-order_execution).
- A **multiprocessor** system executing multiple hardware threads at the same time: In multi-threaded scenarios, you can use two kinds of primitives to deal with synchronization:
  - [memory fences](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html) to ensure memory accesses are made visible to other CPUs in the right order.
  - [atomic operations](https://doc.rust-lang.org/std/sync/atomic/index.html) to ensure simultaneous access to the same memory location doesn’t lead to undefined behavior.

### [Higher-level synchronization objects](https://doc.rust-lang.org/std/sync/index.html#higher-level-synchronization-objects)

The following is an overview of the available synchronization objects:

- [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html): Atomically Reference-Counted pointer, which can be used in multithreaded environments to prolong the lifetime of some data until all the threads have finished using it.
- [`Barrier`](https://doc.rust-lang.org/std/sync/struct.Barrier.html): Ensures multiple threads will wait for each other to reach a point in the program, before continuing execution all together.
- [`Condvar`](https://doc.rust-lang.org/std/sync/struct.Condvar.html): Condition Variable, providing the ability to block a thread while waiting for an event to occur.
- [`mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html): Multi-producer, single-consumer queues, used for message-based communication. Can provide a lightweight inter-thread synchronisation mechanism, at the cost of some extra memory.
- [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html): Mutual Exclusion mechanism, which ensures that at most one thread at a time is able to access some data.
- [`Once`](https://doc.rust-lang.org/std/sync/struct.Once.html): Used for thread-safe, one-time initialization of a global variable.
- [`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html): Provides a mutual exclusion mechanism which allows multiple readers at the same time, while allowing only one writer at a time. In some cases, this can be more efficient than a mutex.

## [Modules](https://doc.rust-lang.org/std/sync/index.html#modules)

[atomic](https://doc.rust-lang.org/std/sync/atomic/index.html)  Atomic types

[mpsc](https://doc.rust-lang.org/std/sync/mpsc/index.html)  Multi-producer, single-consumer FIFO queue communication primitives.

## **[Exercise: Basic multithreading](https://github.com/rust-lang/rustlings/blob/master/exercises/threads/threads1.rs)**

修改如下代码：

```rust
// threads1.rs
// Make this compile! Execute `rustlings hint threads1` for hints :)
// The idea is the thread spawned on line 21 is completing jobs while the main thread is
// monitoring progress until 10 jobs are completed. If you see 6 lines
// of "waiting..." and the program ends without timing out when running,
// you've got it :)

// I AM NOT DONE

use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct JobStatus {
    jobs_completed: u32,
}

fn main() {
    let status = Arc::new(JobStatus { jobs_completed: 0 });
    let status_shared = status.clone();
    thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(250));
            status_shared.jobs_completed += 1;
        }
    });
    while status.jobs_completed < 10 {
        println!("waiting... ");
        thread::sleep(Duration::from_millis(500));
    }
}
```

可以看到代码中想要修改 `Arc` 内部的值，可以考虑使用 `AtomicU32`。修改后的代码如下：

```rust
// threads1.rs
// Make this compile! Execute `rustlings hint threads1` for hints :)
// The idea is the thread spawned on line 21 is completing jobs while the main thread is
// monitoring progress until 10 jobs are completed. If you see 6 lines
// of "waiting..." and the program ends without timing out when running,
// you've got it :)

// I AM NOT DONE

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};

struct JobStatus {
    jobs_completed: AtomicU32,
}

fn main() {
    let status = Arc::new(JobStatus { jobs_completed: AtomicU32::new(0) });
    let status_shared = Arc::clone(&status);
    thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(250));
            status_shared.jobs_completed.fetch_add(1, Ordering::SeqCst);
        }
    });
    while status.jobs_completed.load(Ordering::Relaxed) < 10 {
        println!("waiting... ");
        thread::sleep(Duration::from_millis(500));
    }
}
```

