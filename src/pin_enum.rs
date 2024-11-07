use pin_project_lite::pin_project;
use std::pin::Pin;

pin_project! {
    #[project = EnumProj]
    enum MyEnum {
        Variant1 {
            #[pin]
            field1: String,
        },
        Variant2 {
            field2: u32,
        },
    }
}

impl MyEnum {
    fn get_field1(self: Pin<&mut Self>) -> Option<Pin<&mut String>> {
        // 访问枚举的固定投影
        match self.project() {
            EnumProj::Variant1 { field1 } => Some(field1),
            EnumProj::Variant2 {.. } => None,
        }
    }
}

fn main() {
    let my_enum = MyEnum::Variant1 {
        field1: String::from("Hello, enum!"),
    };

    let mut pinned = Box::pin(my_enum);

    if let Some(field1) = pinned.as_mut().get_field1() {
        println!("{}", field1);
    }
}
