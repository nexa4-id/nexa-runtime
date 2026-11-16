use nexa_runtime::{compiler,Vm};
fn run(s:&str)->Vec<String>{let c=compiler::compile(s).unwrap();let mut vm=Vm::new(2).unwrap();vm.run(&c).unwrap().to_vec()}
#[test]fn if_else(){assert_eq!(run("fn main(){if(3>2){print(1);}else{print(0);}}"),vec!["1".to_string()]);}
#[test]fn while_loop(){assert_eq!(run("fn main(){let i=0;while(i<3){print(i);i=i+1;}}"),vec!["0".to_string(),"1".to_string(),"2".to_string()]);}
#[test]fn functions_and_return(){assert_eq!(run("fn twice(x){return x*2;} fn main(){print(twice(21));}"),vec!["42".to_string()]);}
#[test]fn recursive_function(){assert_eq!(run("fn fact(n){if(n<=1){return 1;}else{return n*fact(n-1);}} fn main(){print(fact(5));}"),vec!["120".to_string()]);}
