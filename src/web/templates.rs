//! Template rendering with Tera

use anyhow::Result;
use serde::Serialize;
use tera::{Context, Tera};

/// Template renderer
pub struct Templates {
    tera: Tera,
}

impl Templates {
    /// Create a new template renderer with embedded templates
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Add base template
        tera.add_raw_template("base.html", include_str!("../templates/base.html"))?;

        // Add page templates
        tera.add_raw_template("index.html", include_str!("../templates/index.html"))?;
        tera.add_raw_template("search.html", include_str!("../templates/search.html"))?;
        tera.add_raw_template("about.html", include_str!("../templates/about.html"))?;
        tera.add_raw_template("usage.html", include_str!("../templates/usage.html"))?;
        tera.add_raw_template(
            "preferences.html",
            include_str!("../templates/preferences.html"),
        )?;
        tera.add_raw_template("stats.html", include_str!("../templates/stats.html"))?;

        // Add component templates
        tera.add_raw_template(
            "components/result.html",
            include_str!("../templates/components/result.html"),
        )?;
        tera.add_raw_template(
            "components/answer.html",
            include_str!("../templates/components/answer.html"),
        )?;
        tera.add_raw_template(
            "components/infobox.html",
            include_str!("../templates/components/infobox.html"),
        )?;
        tera.add_raw_template(
            "components/pagination.html",
            include_str!("../templates/components/pagination.html"),
        )?;

        Ok(Self { tera })
    }

    /// Render a template with context
    pub fn render(&self, template: &str, context: &impl Serialize) -> Result<String> {
        let ctx = Context::from_serialize(context)?;
        Ok(self.tera.render(template, &ctx)?)
    }

    /// Render a template with a Tera Context
    pub fn render_with_context(&self, template: &str, context: &Context) -> Result<String> {
        Ok(self.tera.render(template, context)?)
    }

    /// Create a new context
    pub fn context() -> Context {
        Context::new()
    }
}
