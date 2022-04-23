extern crate proc_macro;

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(RenderGraphLabel)]
pub fn derive_render_graph_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_graph").into());
    // trait_path.segments.push(format_ident!("graph").into());
    trait_path
        .segments
        .push(format_ident!("RenderGraphLabel").into());
    derive_label(input, &trait_path)
}

#[proc_macro_derive(RenderNodeLabel)]
pub fn derive_render_node_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_render_path();
    trait_path
        .segments
        .push(format_ident!("render_graph").into());
    // trait_path.segments.push(format_ident!("node").into());
    trait_path
        .segments
        .push(format_ident!("RenderNodeLabel").into());
    derive_label(input, &trait_path)
}

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_render")
}
