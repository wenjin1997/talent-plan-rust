use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
enum Move {
    Up(i32),
    Down(i32),
    Right(i32),
    Left(i32),
}

fn main(){
    // 序列化之前的值
    let a = Move::Up(56);

    // Vec<u8>
    let vec_buffer: Vec<u8>;
    // 用 ron 的方法进行序列化
    vec_buffer = Vec::from(ron::to_string(&a).unwrap());

    println!("vec_buffer: {:?}", vec_buffer); // vec_buffer: [85, 112, 40, 53, 54, 41]

    let s = std::str::from_utf8(&vec_buffer).unwrap();
    println!("序列化之后的字符串： {}", s); // 序列化之后的字符串： Up(56)
}