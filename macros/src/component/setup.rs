use bq_html::conventions::ComponentOptions;
use syn::{Ident, ItemFn, LitStr, ext::IdentExt};

pub fn attach(func: &mut ItemFn, options: &ComponentOptions) -> syn::Result<()> {
    if !bq_html::setup::has_bare_setup(&func.block) {
        let option = options
            .eager
            .as_ref()
            .map(Ident::span)
            .or_else(|| options.setup.as_ref().map(LitStr::span));
        if let Some(span) = option {
            return Err(syn::Error::new(
                span,
                format!(
                    "`{}` has no bare `bq-setup`, so it has no setup to configure; add `bq-setup` to the element its setup attaches to",
                    func.sig.ident.unraw()
                ),
            ));
        }
    }
    bq_html::setup::mark(&mut func.block, &options.name(&func.sig.ident));
    Ok(())
}
