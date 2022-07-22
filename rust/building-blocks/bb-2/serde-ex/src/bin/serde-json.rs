use std::fs::{File};
use std::io::{BufReader, BufWriter, Write};
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
    let f = File::open("move_to_json.txt").unwrap();

    // 得到 json 的字符串
    let json = serde_json::to_string(&a).unwrap();

    // 将 json 写入文件
    let mut writer =BufWriter::new(&f);
    writer.write(json.as_ref()).unwrap();

    // 反序列化，将文件中的 json 读出来再反序列化
    let reader = BufReader::new(&f);
    let b: Move = serde_json::from_reader(reader).unwrap();

    println!("序列化前： a is {:?}", a);
    println!("反序列化后： b is {:?}", b);

    // 打印结果：
    /*
        序列化前： a is Up(56)
        反序列化后： b is Up(56)
     */
}