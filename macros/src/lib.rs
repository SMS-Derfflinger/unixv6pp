extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Error, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Signature, Type,
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
};

struct ClassCompat {
    class_name: Box<Type>,
    funcs: Vec<ImplItemFn>,
}

impl Parse for ClassCompat {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_impl = input.parse::<ItemImpl>()?;
        let mut funcs = vec![];

        for item in item_impl.items {
            let ImplItem::Fn(func) = item else {
                return Err(Error::new(item.span(), "only function definitions allowed"));
            };

            funcs.push(func);
        }

        Ok(ClassCompat {
            class_name: item_impl.self_ty,
            funcs,
        })
    }
}

fn define_func_one(class_name: &Type, impl_item: ImplItemFn) -> TokenStream {
    let span = impl_item.span();
    let ImplItemFn {
        attrs,
        vis,
        sig,
        block,
        ..
    } = &impl_item;
    let Signature {
        ident,
        inputs,
        output,
        ..
    } = &sig;

    let args = {
        let mut inputs_iter = inputs.iter();
        let selfness = match inputs_iter.next() {
            Some(FnArg::Receiver(recv)) => {
                let mutability = recv.mutability;
                quote_spanned! {recv.span()=>
                    this: & #mutability #class_name,
                }
            }
            Some(FnArg::Typed(ty)) => {
                quote_spanned! {ty.span()=>
                    #ty,
                }
            }
            None => quote! {},
        };

        let span = inputs.span();
        quote_spanned! {span=>
            #selfness #(#inputs_iter),*
        }
    };

    let prefixed_ident = Ident::new(
        &format!("{}_{}", quote!(#class_name).to_string(), ident.to_string()),
        ident.span(),
    );

    quote_spanned! {span=>
        #[no_mangle]
        #(#attrs)*
        #vis extern "C" fn #prefixed_ident(#args) #output #block
    }
}

fn define_class_compat_impl(items: TokenStream) -> TokenStream {
    let ClassCompat { class_name, funcs } = parse2::<ClassCompat>(items).expect("syntax error");

    let defs = funcs
        .into_iter()
        .map(|func| define_func_one(&class_name, func));

    quote! {
        #(#defs)*
    }
}

#[proc_macro]
pub fn define_class_compat(items: proc_macro::TokenStream) -> proc_macro::TokenStream {
    define_class_compat_impl(items.into()).into()
}
