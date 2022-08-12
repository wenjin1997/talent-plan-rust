# PNA Rust Project 4: Concurrency and parallelism

Rust 中`std::sync::atomic::Ordering` 和 C++20 里含义一样，参考 [程序员的自我修养（⑫）：C++ 的内存顺序·中](https://liam.page/2021/12/11/memory-order-cpp-02/)。

```rust
#[non_exhaustive]
pub enum Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
}
```

| 标记                   | 作用                                                         |
| :--------------------- | :----------------------------------------------------------- |
| `memory_order_relaxed` | 宽松操作：仅保证原子操作自身的原子性，对其他读写操作不做任何同步，亦无顺序上的限制。 |
| `memory_order_consume` | 打上此标记的 load 操作对相关内存位置施加[消费操作（consume operation）](https://liam.page/2021/06/06/memory-order-cpp-01/#消费操作（consume-operation）)：当前线程中，所有依赖当前 load 操作读取的值的读写操作不得重排序至当前操作之前。因此，其他线程中相同原子变量释放操作（release operation）依赖的变量的写入，对当前线程是可见的。多数平台上，该标记仅影响编译器优化。 |
| `memory_order_acquire` | 打上此标记的 load 操作对相关内存位置施加[占有操作（aquire operation）](https://liam.page/2021/06/06/memory-order-cpp-01/#占有操作（aquire-operation）)：当前线程中，所有读写操作不得重排序至当前操作之前。因此，其他线程中相同原子变量释放操作（release operation）之前的写入，对当前线程是可见的。 |
| `memory_order_release` | 打上此标记的 store 操作对相关内存位置施加[释放操作（release operation）](https://liam.page/2021/06/06/memory-order-cpp-01/#释放操作（release-operation）)：当前线程中，所有读写操作不得重排至当前操作之后。因此，当前操作所在线程之前的写入操作，在其他线程中，对该原子变量施加占有操作（aquire operation）之后是可见的。也因此，当前操作所在线程中，当前操作所依赖的写入操作，在其他线程中，对该原子变量施加消费操作（consume operation）之后是可见的。 |
| `memory_order_acq_rel` | 打上此标记的 read-modify-write 操作既是占有操作（aquire operation）又是释放操作（release operation）：当前线程中的读写操作不能重排至当前操作之后（如果原本在之前），亦不能重排至当前操作之前（如果原本在之后）。因此，其他线程中相同原子变量释放操作（release operation）之前的写入，对当前 modification 是可见的；该 modification 对其他线程中相同原子变量占有操作（aquire operation）之后亦是可见的。 |
| `memory_order_seq_cst` | 打上此标记的 load 操作对相关内存位置施加占有操作（aquire operation）；打上此标记的 store 操作对相关内存位置施加释放操作（release operation）；打上此标记的 read-modify-write 对相关内存位置施加占有操作（aquire operation）和释放操作（release operation）。此外，对所有线程来说，所有打上该标记的写操作，存在一个全局修改顺序（尽管具体顺序在执行时才确定）。也就是说，对于所有线程来说，看见的这些写操作的顺序是一致的。 |

## Module [std](https://doc.rust-lang.org/std/index.html)::[cell](https://doc.rust-lang.org/std/cell/index.html#)

`Cell<T>` 和 `RefCell<T>` 都不是线程安全的，它们没有实现 `Sync`。`Cell<T>`和`RefCell<T>`提供了内部可变性。

`Cell<T>` 通过将值 move 进和 move 出 `Cell<T>` 来实现内部可变性。如果要使用引用而不是值，必须使用 `RefCell<T>` 类型，在改变之前获得写的锁。

`Cell<T>` 提供了检索和更改当前内部值的方法：

* 对于实现 Copy 的类型，get 方法检索当前内部值。
* 实现 Default 的类型，take 方法将当前内部值替换为 `Default::default()` 并返回替换后的值。
* 对于所有类型，replace 方法替换当前内部值并返回替换值，而 into_inner 方法消耗 `Cell<T>` 并返回内部值。此外，set 方法替换内部值，删除替换值。

`RefCell<T>` 使用Rust的生命周期来实现“动态借用”，这是一个可以声明对内部值的临时、独占、可变访问的过程。`RefCell<T>` 的借用是在“运行时”跟踪的，这与 Rust 的本地引用类型不同，后者在编译时完全静态跟踪。因为 `RefCell<T>` 借用是动态的，所以可以尝试借用一个已经可变借用的值；当这种情况发生时，它会导致线程恐慌。

什么时候选择内部可变性呢？有三种情况：

* 在一些不可变的内部引入可变性。
* 逻辑上不可变方法的实现。
* Clone 的可变实现。

### [Introducing mutability ‘inside’ of something immutable](https://doc.rust-lang.org/std/cell/index.html#introducing-mutability-inside-of-something-immutable)

```rust
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

fn main() {
    let shared_map: Rc<RefCell<_>> = Rc::new(RefCell::new(HashMap::new()));
    // Create a new block to limit the scope of the dynamic borrow
    {
        let mut map: RefMut<_> = shared_map.borrow_mut();
        map.insert("africa", 92388);
        map.insert("kyoto", 11837);
        map.insert("piccadilly", 11826);
        map.insert("marbles", 38);
    }

    // Note that if we had not let the previous borrow of the cache fall out
    // of scope then the subsequent borrow would cause a dynamic thread panic.
    // This is the major hazard of using `RefCell`.
    let total: i32 = shared_map.borrow().values().sum();
    println!("{total}");
}
```

注意这里用的 `Rc<T>`，如果要在多线程中使用 `Arc<T>` ，要考虑使用 `RwLock<T>` 或者 `Mutex<T>` 。

### [Implementation details of logically-immutable methods](https://doc.rust-lang.org/std/cell/index.html#implementation-details-of-logically-immutable-methods)

```rust
use std::cell::RefCell;

struct Graph {
    edges: Vec<(i32, i32)>,
    span_tree_cache: RefCell<Option<Vec<(i32, i32)>>>
}

impl Graph {
    fn minimum_spanning_tree(&self) -> Vec<(i32, i32)> {
        self.span_tree_cache.borrow_mut()
            .get_or_insert_with(|| self.calc_span_tree())
            .clone()
    }

    fn calc_span_tree(&self) -> Vec<(i32, i32)> {
        // Expensive computation goes here
        vec![]
    }
}
```

### [Mutating implementations of `Clone`](https://doc.rust-lang.org/std/cell/index.html#mutating-implementations-of-clone)

```rust
use std::cell::Cell;
use std::ptr::NonNull;
use std::process::abort;
use std::marker::PhantomData;

struct Rc<T: ?Sized> {
    ptr: NonNull<RcBox<T>>,
    phantom: PhantomData<RcBox<T>>,
}

struct RcBox<T: ?Sized> {
    strong: Cell<usize>,
    refcount: Cell<usize>,
    value: T,
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Rc<T> {
        self.inc_strong();
        Rc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

trait RcBoxPtr<T: ?Sized> {

    fn inner(&self) -> &RcBox<T>;

    fn strong(&self) -> usize {
        self.inner().strong.get()
    }

    fn inc_strong(&self) {
        self.inner()
            .strong
            .set(self.strong()
                     .checked_add(1)
                     .unwrap_or_else(|| abort() ));
    }
}

impl<T: ?Sized> RcBoxPtr<T> for Rc<T> {
   fn inner(&self) -> &RcBox<T> {
       unsafe {
           self.ptr.as_ref()
       }
   }
}
```