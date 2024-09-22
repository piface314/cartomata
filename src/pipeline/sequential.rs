use crate::data::{Card, Predicate};
use crate::decode::Decoder;
use crate::error::Result;
use crate::image::ImgBackend;
use crate::layer::RenderContext;
use crate::pipeline::{Pipeline, Visitor};
use crate::template::Template;

impl<C, T, V> Pipeline<C, T, V>
where
    C: Card,
    T: Template<C>,
    V: Visitor<C, T>,
{
    pub fn run(self, source_key: T::SourceKey, filter: Option<Predicate>) -> (T, V) {
        let template = self.template;
        let visitor = self.visitor;
        let result = Self::run_internal(&template, &visitor, source_key, filter);
        visitor.on_finish(&template, 0, &result);
        (template, visitor)
    }

    fn run_internal(
        template: &T,
        visitor: &V,
        source_key: T::SourceKey,
        filter: Option<Predicate>,
    ) -> Result<()> {
        visitor.on_start(&template, 0);
        let mut source = template.source(source_key)?;
        let decoder = template.decoder()?;
        let font_map = template.fonts();
        let img_map = template.resources();
        let backend = ImgBackend::new()?;
        let ctx = RenderContext { backend: &backend, font_map, img_map };
        source
            .read(filter)?
            .filter(|card_res| visitor.on_read(template, card_res))
            .enumerate()
            .filter_map(|(i, card_res)| match card_res {
                Ok(card) => Some((i, card)),
                Err(e) => {
                    visitor.on_read_err(template, i, e);
                    None
                }
            })
            .for_each(|(i, card)| {
                visitor.on_iter_start(template, 0, i, &card);
                match Self::process(&template, &decoder, &card, &ctx) {
                    Ok(()) => visitor.on_iter_ok(template, 0, i, card),
                    Err(e) => visitor.on_iter_err(template, 0, i, card, e),
                }
            });
        Ok(())
    }

    fn process(template: &T, decoder: &T::Decoder, card: &C, ctx: &RenderContext) -> Result<()> {
        let layers = decoder.decode(card)?;
        let img = layers.render(ctx)?;
        template.output(card, &img, &ctx.backend)?;
        Ok(())
    }
}
