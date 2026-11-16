#[allow(unused)]
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[cfg(any(feature = "is-lp-core", feature = "is-ulp-core"))]
pub fn shared(attr: TokenStream, item: TokenStream) -> TokenStream {
    use parse::Error;
    use proc_macro2::Span;
    use syn::parse;

    let _attrs: Vec<_> = attr.clone().into_iter().collect();
    let items: Vec<_> = item.clone().into_iter().collect();

    // item must be the following format, at the moment:
    // pub static [mut] #ITEM : #TYPE = #INIT_VALUE;

    // Minimum of 8 items, depending on scope of #TY
    // pub static #ITEM: #TY = #INIT;
    // Minimum of 9 items, depending on scope of #TY
    // pub static mut #ITEM: #TY = #INIT;

    let is_pub = items[0].to_string().starts_with("pub");
    let is_static = items[1].to_string().starts_with("static");

    if !is_pub || !is_static {
        return Error::new(Span::call_site(), "Variable must be `pub static`").to_compile_error();
    }

    let is_mut = items[2].to_string().starts_with("mut");
    let is_mut_flag = if is_mut { "mut" } else { "" };

    let var_name = if !is_mut {
        items[2].to_string()
    } else {
        items[3].to_string()
    };

    // after var_name, should go
    // : (colon)
    // #TYPE (1 or more items)
    // = (equals)
    // #INIT (1 item I think? maybe more?)
    // ; (semicolon)

    let mut var_type: String = String::new();
    let start_idx = if !is_mut { 4 } else { 5 };
    let _ = items[start_idx..items.len()]
        .iter()
        .take_while(|i| {
            if i.to_string() == "=" {
                return false;
            }
            var_type = format!("{}{}", var_type, i.to_string());
            true
        })
        .collect::<Vec<_>>();

    let encoded_var_type = var_type
        .as_bytes()
        .iter()
        .map(|x| format!("{:02x}", x))
        .collect::<String>();

    let out: TokenStream = format!(
        r#"#[used]
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "__ULP_${var_name}${encoded_var_type}${is_mut_flag}")]
        {item}"#,
    )
    .parse()
    .unwrap();

    out
}

#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
pub fn load_shared(input: TokenStream, fs: impl Filesystem) -> TokenStream {
    use std::io::Write;

    use object::{File, Object, ObjectSymbol};
    use parse::Error;
    use proc_macro2::Span;
    use syn::{LitStr, parse};

    fn base16_to_utf8(encoded: String) -> Result<String, Box<dyn std::error::Error>> {
        // convert the base16 string into a vector of bytes,
        // which we can then convert back into a UTF-8 string.
        let mut encoded_bytes: Vec<u8> = vec![];
        for a in { 0..encoded.len() }.step_by(2) {
            let x = encoded.as_str();
            match u8::from_str_radix(&x[a..a + 2], 16) {
                Ok(b) => encoded_bytes.push(b),
                Err(e) => return Err(Box::new(e)),
            }
        }
        Ok(String::from_utf8(encoded_bytes).unwrap())
    }

    struct SharedSymbol {
        name: String,
        address: u64,
        enc_type: String,
        is_mut: bool,
        dec_name: String,
        dec_type: String,
    }

    fn decode_symbol(address: u64, name: &str) -> Result<SharedSymbol, Box<dyn std::error::Error>> {
        let parts: Vec<_> = name.split("$").collect();
        // Get the parts we are interested in
        let var_name = parts[1];
        let encoded_var_type = parts[2];
        let is_mut = parts[3].len() > 1;
        // Decode variable type from base16 to utf8
        let var_type = base16_to_utf8(encoded_var_type.to_string())?;
        Ok(SharedSymbol {
            name: name.to_string(),
            dec_name: var_name.to_string(),
            address: address,
            enc_type: encoded_var_type.to_string(),
            dec_type: var_type,
            is_mut: is_mut,
        })
    }

    let lit: LitStr = match syn::parse2(input) {
        Ok(lit) => lit,
        Err(e) => return e.into_compile_error(),
    };

    let elf_file = lit.value();
    if !fs.exists(&elf_file) {
        return Error::new(Span::call_site(), "File not found").to_compile_error();
    }

    let bin_data = fs.read(&elf_file).unwrap();
    let obj_file: File<'_> = match File::parse(&*bin_data) {
        Ok(obj_file) => obj_file,
        Err(e) => {
            return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error();
        }
    };

    // Iterate over shared variables, based on prefix
    let obj_symbols = obj_file
        .symbols()
        .filter(|s| s.name().unwrap().starts_with("__ULP_$"));

    // Decode the symbols
    let mut symbols: Vec<SharedSymbol> = vec![];
    for s in obj_symbols {
        let name = s.name().unwrap();
        match decode_symbol(s.address(), name) {
            Ok(sy) => symbols.push(sy),
            Err(e) => {
                // Return early, if decoding a symbol fails
                return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error();
            }
        };
    }

    // Create the linker script
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(o) => o,
        Err(e) => return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error(),
    };
    println!("out_dir = {}", out_dir);
    let xpath = std::path::PathBuf::from(&(out_dir.clone() + "/lp_shared.x"));
    let mut x = match std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(xpath)
    {
        Ok(f) => f,
        Err(e) => {
            return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error();
        }
    };

    for sym in &symbols {
        let i_addr = sym.address;
        let i_name = sym.name.clone();
        let o_addr = i_addr + 0x5000_0000;
        let o_name = sym.dec_name.clone();

        let xdata = format_args!(
            r#"PROVIDE({i_name} = {o_addr:#x});
PROVIDE({o_name} = {i_name});"#
        );
        println!("{xdata}");

        match x.write_fmt(xdata) {
            Ok(_) => {}
            Err(e) => {
                return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error();
            }
        };
    }
    drop(x);

    // Build the extern variable entires
    let extern_variables: Vec<TokenStream> = symbols
        .iter()
        .map(|sym| {
            let prefix = if sym.is_mut {
                "pub safe static mut"
            } else {
                "pub safe static"
            };

            // Build the code chunk
            let symbol_arg = format!(
                "{} {} : {}; // {}",
                prefix, sym.dec_name, sym.dec_type, sym.enc_type
            );

            // debugging
            eprintln!("{}", symbol_arg);
            symbol_arg.parse().unwrap()
        })
        .collect();

    quote! {
        unsafe extern "Rust" {
            #(#extern_variables)*
        }
    }
}

#[cfg(any(feature = "is-lp-core", feature = "is-ulp-core"))]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    use proc_macro_crate::FoundCrate;
    #[cfg(not(test))]
    use proc_macro_crate::crate_name;
    use proc_macro2::{Ident, Span};
    use syn::{
        FnArg,
        GenericArgument,
        ItemFn,
        PatType,
        PathArguments,
        Type,
        parse::Error,
        spanned::Spanned,
    };

    pub(crate) fn make_magic_symbol_name(args: &Vec<&PatType>) -> String {
        let mut res = String::from("__ULP_MAGIC_");
        for &a in args {
            let t = &a.ty;
            let quoted = to_string(t);
            res.push_str(&quoted);
            res.push('$');
        }

        res
    }

    pub(crate) fn simplename(t: &Type) -> String {
        match t {
            Type::Path(p) => p.path.segments.last().unwrap().ident.to_string(),
            _ => String::new(),
        }
    }

    pub(crate) fn extract_pin(ty: &Type) -> u8 {
        let mut res = 255u8;
        if let Type::Path(p) = ty {
            let segment = p.path.segments.last().unwrap();
            if let PathArguments::AngleBracketed(g) = &segment.arguments {
                for arg in &g.args {
                    match arg {
                        GenericArgument::Type(t) => {
                            res = extract_pin(t);
                        }
                        GenericArgument::Const(c) => {
                            res = quote! { #c }.to_string().parse().unwrap();
                        }
                        _ => (),
                    }
                }
            }
        }

        res
    }

    // This is a specialized implementation - won't fit other use-cases
    fn to_string(ty: &Type) -> String {
        let mut res = String::new();
        if let Type::Path(p) = ty {
            let segment = p.path.segments.last().unwrap();
            res.push_str(&segment.ident.to_string());

            if let PathArguments::AngleBracketed(g) = &segment.arguments {
                res.push('<');
                let mut pushed = false;
                for arg in &g.args {
                    if pushed {
                        res.push(',');
                    }

                    match arg {
                        GenericArgument::Type(t) => {
                            pushed = true;
                            res.push_str(&to_string(t));
                        }
                        GenericArgument::Const(c) => {
                            pushed = true;
                            res.push_str(&quote! { #c }.to_string());
                        }
                        _ => (),
                    }
                }
                res.push('>');
            }
        }

        res
    }

    #[cfg(not(test))]
    let found_crate = crate_name("esp-lp-hal").expect("esp-lp-hal is present in `Cargo.toml`");

    #[cfg(test)]
    let found_crate = FoundCrate::Itself;

    let hal_crate = match found_crate {
        FoundCrate::Itself => quote!(esp_lp_hal),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    };

    if !args.is_empty() {
        return Error::new(Span::call_site(), "This attribute accepts no arguments")
            .to_compile_error();
    }

    let f: ItemFn = crate::unwrap_or_compile_error!(syn::parse2(input));

    let mut argument_types = Vec::new();
    let mut create_peripheral = Vec::new();

    let mut used_pins: Vec<u8> = Vec::new();

    for (num, arg) in f.sig.inputs.iter().enumerate() {
        let param_name = format_ident!("param{}", num);
        match arg {
            FnArg::Receiver(_) => {
                return Error::new(arg.span(), "invalid argument").to_compile_error();
            }
            FnArg::Typed(t) => {
                match simplename(&t.ty).as_str() {
                    "Output" => {
                        let pin = extract_pin(&t.ty);
                        if used_pins.contains(&pin) {
                            return Error::new(arg.span(), "duplicate pin").to_compile_error();
                        }
                        used_pins.push(pin);
                        create_peripheral.push(quote!(
                            let mut #param_name = unsafe { the_hal::gpio::conjure_output().unwrap() };
                        ));
                    }
                    "OutputOpenDrain" => {
                        let pin = extract_pin(&t.ty);
                        if used_pins.contains(&pin) {
                            return Error::new(arg.span(), "duplicate pin").to_compile_error();
                        }
                        used_pins.push(pin);
                        create_peripheral.push(quote!(
                            let mut #param_name = unsafe { the_hal::gpio::conjure_output_open_drain().unwrap() };
                        ));
                    }
                    "Input" => {
                        let pin = extract_pin(&t.ty);
                        if used_pins.contains(&pin) {
                            return Error::new(arg.span(), "duplicate pin").to_compile_error();
                        }
                        used_pins.push(pin);
                        create_peripheral.push(quote!(
                            let mut #param_name = unsafe { the_hal::gpio::conjure_input().unwrap() };
                        ));
                    }
                    "LpUart" => {
                        create_peripheral.push(quote!(
                            let mut #param_name = unsafe { the_hal::uart::conjure() };
                        ));
                    }
                    "LpI2c" => {
                        create_peripheral.push(quote!(
                            let mut #param_name = unsafe { the_hal::i2c::conjure() };
                        ));
                    }
                    _ => {
                        return Error::new(arg.span(), "invalid argument to main")
                            .to_compile_error();
                    }
                }
                argument_types.push(t);
            }
        }
    }

    let magic_symbol_name = make_magic_symbol_name(&argument_types);

    let param_names: Vec<Ident> = argument_types
        .into_iter()
        .enumerate()
        .map(|(num, _)| format_ident!("param{}", num))
        .collect();

    quote!(
        #[allow(non_snake_case)]
        #[unsafe(export_name = "main")]
        pub fn __risc_v_rt__main() {
            #[unsafe(export_name = #magic_symbol_name)]
            static ULP_MAGIC: [u32; 0] = [0u32; 0];

            unsafe { ULP_MAGIC.as_ptr().read_volatile(); }

            use #hal_crate as the_hal;
            #(
                #create_peripheral
            )*

            main(#(#param_names),*);
        }

        #f
    )
}

#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
pub fn load_lp_code(input: TokenStream, fs: impl Filesystem) -> TokenStream {
    use object::{File, Object, ObjectSection, ObjectSymbol, Section, SectionFlags};
    use parse::Error;
    use proc_macro_crate::{FoundCrate, crate_name};
    use proc_macro2::Span;
    use syn::{Ident, LitStr, parse};

    let hal_crate = if cfg!(any(feature = "is-lp-core", feature = "is-ulp-core")) {
        crate_name("esp-lp-hal")
    } else {
        crate_name("esp-hal")
    };

    let hal_crate = if let Ok(FoundCrate::Name(ref name)) = hal_crate {
        let ident = Ident::new(name, proc_macro2::Span::call_site());
        quote!( #ident )
    } else {
        quote!(crate)
    };

    let lit: LitStr = match syn::parse2(input) {
        Ok(lit) => lit,
        Err(e) => return e.into_compile_error(),
    };

    let elf_file = lit.value();

    if !fs.exists(&elf_file) {
        return Error::new(Span::call_site(), "File not found").to_compile_error();
    }

    let bin_data = fs.read(&elf_file).unwrap();
    let obj_file = match File::parse(&*bin_data) {
        Ok(obj_file) => obj_file,
        Err(e) => {
            return Error::new(Span::call_site(), format!("Error: {}", e)).to_compile_error();
        }
    };
    let sections = obj_file.sections();

    let mut sections: Vec<Section> = sections
        .into_iter()
        .filter(|section| match section.flags() {
            SectionFlags::Elf { sh_flags: sh } => (sh & u64::from(object::elf::SHF_ALLOC)) != 0,
            _ => false,
        })
        .collect();
    sections.sort_by(|a, b| a.address().partial_cmp(&b.address()).unwrap());

    let mut binary: Vec<u8> = Vec::new();
    let mut last_address = if cfg!(feature = "has-lp-core") {
        0x5000_0000
    } else {
        0x0
    };

    if sections.is_empty() {
        return Error::new(
            Span::call_site(),
            "Given file doesn't seem to have any allocatable sections.",
        )
        .to_compile_error();
    } else if sections[0].address() < last_address {
        return Error::new(
            Span::call_site(),
            format!(
                "First section address is below expected base address (expected >= 0x{:x}, got 0x{:x})",
                last_address,
                sections[0].address()
            ),
        )
        .to_compile_error();
    }

    for section in sections {
        if section.address() > last_address {
            let fill = section.address() - last_address;
            binary.extend(std::iter::repeat_n(0, fill as usize));
        }

        binary.extend_from_slice(section.data().unwrap());
        last_address = section.address() + section.size();
    }

    let magic_symbol = obj_file
        .symbols()
        .find(|s| s.name().unwrap().starts_with("__ULP_MAGIC_"));

    let magic_symbol = if let Some(magic_symbol) = magic_symbol {
        magic_symbol.name().unwrap()
    } else {
        return Error::new(
            Span::call_site().into(),
            "Given file doesn't seem to be an LP/ULP core application.",
        )
        .to_compile_error();
    };

    let magic_symbol = magic_symbol.trim_start_matches("__ULP_MAGIC_");
    let args: Vec<proc_macro2::TokenStream> = magic_symbol
        .split("$")
        .map(|t| {
            let t = if t.contains("OutputOpenDrain") {
                t.replace("OutputOpenDrain", "LowPowerOutputOpenDrain")
            } else {
                t.replace("Output", "LowPowerOutput")
            };
            let t = t.replace("Input", "LowPowerInput");
            t.parse().unwrap()
        })
        .filter(|v: &proc_macro2::TokenStream| !v.is_empty())
        .collect();
    let arg_names = (0..args.len())
        .map(|i| format_ident!("arg_{i}"))
        .collect::<Vec<_>>();

    #[cfg(feature = "has-lp-core")]
    let imports = quote! {
        use #hal_crate::lp_core::LpCore;
        use #hal_crate::lp_core::LpCoreWakeupSource;
        use #hal_crate::gpio::lp_io::{LowPowerInput, LowPowerOutput, LowPowerOutputOpenDrain};
        use #hal_crate::uart::lp_uart::LpUart;
        use #hal_crate::i2c::lp_i2c::LpI2c;
    };
    #[cfg(feature = "has-ulp-core")]
    let imports = quote! {
        use #hal_crate::lp_core::UlpCore as LpCore;
        use #hal_crate::lp_core::UlpCoreWakeupSource as LpCoreWakeupSource;
        use #hal_crate::gpio::lp_io::{LowPowerInput, LowPowerOutput, LowPowerOutputOpenDrain};
    };

    #[cfg(feature = "has-lp-core")]
    let rtc_code_start = quote! { _rtc_fast_data_start };
    #[cfg(feature = "has-ulp-core")]
    let rtc_code_start = quote! { _rtc_slow_data_start };

    quote! {
        {
            #imports

            struct LpCoreCode {}

            static LP_CODE: &[u8] = &[#(#binary),*];

            unsafe extern "C" {
                static #rtc_code_start: u32;
            }

            unsafe {
                core::ptr::copy_nonoverlapping(LP_CODE as *const _ as *const u8, &#rtc_code_start as *const u32 as *mut u8, LP_CODE.len());
            }

            impl LpCoreCode {
                pub fn run(
                    &self,
                    lp_core: &mut LpCore,
                    wakeup_source: LpCoreWakeupSource,
                    #(#arg_names: #args),*
                ) {
                    #(core::mem::forget(#arg_names);)*
                    lp_core.run(wakeup_source);
                }
            }

            LpCoreCode {}
        }
    }
}

#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
pub(crate) trait Filesystem {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>>;

    fn exists(&self, path: &str) -> bool;
}

#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
pub(crate) struct RealFilesystem;

#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
impl Filesystem for RealFilesystem {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
    struct TestFilesystem;

    #[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
    impl Filesystem for TestFilesystem {
        fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
            if path == "doesnt_exist.elf" {
                Err(std::io::ErrorKind::NotFound.into())
            } else if path == "bad.elf" {
                Ok(Vec::from(&[1, 2, 3, 4]))
            } else {
                std::fs::read("./testdata/ulp_code.elf")
            }
        }

        fn exists(&self, path: &str) -> bool {
            if path == "doesnt_exist.elf" {
                false
            } else {
                true
            }
        }
    }

    #[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
    #[test]
    fn test_load_lp_code_file_not_found() {
        let result = load_lp_code(quote::quote! {"doesnt_exist.elf"}.into(), TestFilesystem);

        assert_eq!(
            result.to_string(),
            quote::quote! {
                ::core::compile_error! { "File not found" }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
    #[test]
    fn test_load_lp_code_bad() {
        let result = load_lp_code(quote::quote! {"bad.elf"}.into(), TestFilesystem);

        assert_eq!(
            result.to_string(),
            quote::quote! {
                ::core::compile_error! { "Error: Could not read file magic" }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "has-lp-core", feature = "has-ulp-core"))]
    #[test]
    fn test_load_lp_code_basic() {
        let result = load_lp_code(quote::quote! {"good.elf"}.into(), TestFilesystem);

        assert_eq!(
            result.to_string(),
            quote::quote! {
                {
                    use crate::lp_core::LpCore;
                    use crate::lp_core::LpCoreWakeupSource;
                    use crate::gpio::lp_io::{LowPowerInput, LowPowerOutput, LowPowerOutputOpenDrain};
                    use crate::uart::lp_uart::LpUart;
                    use crate::i2c::lp_i2c::LpI2c;

                    struct LpCoreCode {}
                    static LP_CODE: &[u8] = &[
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8, 19u8, 0u8, 0u8, 0u8,
                        9u8, 160u8, 23u8, 65u8, 0u8, 0u8, 19u8, 1u8, 225u8, 247u8, 151u8, 0u8, 0u8, 0u8, 231u8,
                        128u8, 160u8, 0u8, 1u8, 160u8, 55u8, 5u8, 11u8, 96u8, 3u8, 37u8, 5u8, 64u8, 17u8,
                        137u8, 9u8, 201u8, 55u8, 5u8, 0u8, 80u8, 183u8, 53u8, 49u8, 1u8, 147u8, 133u8, 5u8,
                        208u8, 35u8, 46u8, 181u8, 22u8, 151u8, 0u8, 0u8, 0u8, 231u8, 128u8, 192u8, 11u8, 129u8,
                        67u8, 183u8, 40u8, 11u8, 96u8, 55u8, 40u8, 0u8, 80u8, 137u8, 66u8, 55u8, 3u8, 0u8,
                        80u8, 133u8, 3u8, 35u8, 32u8, 120u8, 0u8, 35u8, 162u8, 88u8, 0u8, 131u8, 39u8, 195u8,
                        23u8, 243u8, 37u8, 0u8, 184u8, 243u8, 38u8, 0u8, 176u8, 115u8, 38u8, 0u8, 184u8, 227u8,
                        154u8, 197u8, 254u8, 133u8, 131u8, 51u8, 6u8, 208u8, 64u8, 179u8, 54u8, 208u8, 0u8,
                        179u8, 5u8, 176u8, 64u8, 149u8, 141u8, 243u8, 38u8, 0u8, 184u8, 115u8, 39u8, 0u8,
                        176u8, 115u8, 37u8, 0u8, 184u8, 227u8, 154u8, 166u8, 254u8, 50u8, 151u8, 174u8, 150u8,
                        51u8, 53u8, 199u8, 0u8, 54u8, 149u8, 179u8, 182u8, 231u8, 0u8, 51u8, 53u8, 160u8, 0u8,
                        85u8, 141u8, 113u8, 221u8, 35u8, 164u8, 88u8, 0u8, 3u8, 38u8, 195u8, 23u8, 243u8, 37u8,
                        0u8, 184u8, 243u8, 38u8, 0u8, 176u8, 115u8, 37u8, 0u8, 184u8, 227u8, 154u8, 165u8,
                        254u8, 5u8, 130u8, 179u8, 7u8, 208u8, 64u8, 51u8, 53u8, 208u8, 0u8, 179u8, 5u8, 176u8,
                        64u8, 137u8, 141u8, 243u8, 38u8, 0u8, 184u8, 115u8, 39u8, 0u8, 176u8, 115u8, 37u8, 0u8,
                        184u8, 227u8, 154u8, 166u8, 254u8, 62u8, 151u8, 174u8, 150u8, 51u8, 53u8, 247u8, 0u8,
                        54u8, 149u8, 179u8, 54u8, 230u8, 0u8, 51u8, 53u8, 160u8, 0u8, 85u8, 141u8, 113u8,
                        221u8, 185u8, 191u8, 55u8, 5u8, 0u8, 80u8, 3u8, 32u8, 197u8, 23u8, 151u8, 0u8, 0u8,
                        0u8, 231u8, 128u8, 64u8, 244u8, 0u8, 36u8, 244u8, 0u8
                    ];
                    unsafe extern "C" {
                        static _rtc_fast_data_start: u32;
                    }
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            LP_CODE as *const _ as *const u8,
                            &_rtc_fast_data_start as *const u32 as *mut u8,
                            LP_CODE.len()
                        );
                    }
                    impl LpCoreCode {
                        pub fn run(
                            &self,
                            lp_core: &mut LpCore,
                            wakeup_source: LpCoreWakeupSource,
                            arg_0: LowPowerOutput<1>
                        ) {
                            core::mem::forget(arg_0);
                            lp_core.run(wakeup_source);
                        }
                    }
                    LpCoreCode {}
                }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "is-lp-core"))]
    #[test]
    fn test_lp_entry_basic() {
        let result = entry(
            quote::quote! {}.into(),
            quote::quote! {
                fn main(){}
            }
            .into(),
        );

        assert_eq!(
            result.to_string(),
            quote::quote! {
                #[allow(non_snake_case)]
                #[unsafe(export_name = "main")]
                pub fn __risc_v_rt__main () {
                    #[unsafe(export_name = "__ULP_MAGIC_")]
                    static ULP_MAGIC: [u32;0] = [0u32;0];
                    unsafe {
                        ULP_MAGIC.as_ptr().read_volatile ();
                    }

                    use esp_lp_hal as the_hal;
                    main ();
                }

                fn main () { }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "is-lp-core"))]
    #[test]
    fn test_lp_entry_with_params() {
        let result = entry(
            quote::quote! {}.into(),
            quote::quote! {
                fn main(mut gpio1: Output<1>, mut i2c: LpI2c, mut uart: LpUart){}
            }
            .into(),
        );

        assert_eq!(
            result.to_string(),
            quote::quote! {
                #[allow(non_snake_case)]
                #[unsafe(export_name = "main")]
                pub fn __risc_v_rt__main () {
                    # [unsafe (export_name = "__ULP_MAGIC_Output<1>$LpI2c$LpUart$")]
                    static ULP_MAGIC: [u32;0] = [0u32;0];
                    unsafe { ULP_MAGIC.as_ptr().read_volatile();
                }

                use esp_lp_hal as the_hal ;
                let mut param0 = unsafe { the_hal::gpio::conjure_output().unwrap() };
                let mut param1 = unsafe { the_hal::i2c::conjure() };
                let mut param2 = unsafe { the_hal::uart::conjure() };
                main (param0 , param1 , param2) ; }

                fn main (mut gpio1 : Output < 1 > , mut i2c : LpI2c , mut uart : LpUart) { }
            }
            .to_string()
        );
    }

    #[cfg(any(feature = "is-lp-core"))]
    #[test]
    fn test_lp_entry_non_empty_args() {
        let result = entry(
            quote::quote! { foo }.into(),
            quote::quote! {
                fn main(){}
            }
            .into(),
        );

        assert_eq!(
            result.to_string(),
            quote::quote! {
                ::core::compile_error!{ "This attribute accepts no arguments" }
            }
            .to_string()
        );
    }
}
