use nexa_runtime::{compiler,Vm};
fn run(source:&str)->Vec<String>{let code=compiler::compile(source).unwrap();let mut vm=Vm::new(2).unwrap();vm.run(&code).unwrap().to_vec()}
#[test]fn if_else_and_comparison(){assert_eq!(run("fn main(){if(3>2){print(1);}else{print(0);}}"),vec!["1"]);}
#[test]fn while_loop(){assert_eq!(run("fn main(){let i=0;while(i<3){print(i);i=i+1;}}"),vec!["0","1","2"]);}
#[test]fn function_arguments_and_return(){assert_eq!(run("fn twice(x){return x*2;} fn main(){print(twice(21));}"),vec!["42"]);}
#[test]fn recursive_function(){assert_eq!(run("fn fact(n){if(n<=1){return 1;}else{return n*fact(n-1);}} fn main(){print(fact(5));}"),vec!["120"]);}
