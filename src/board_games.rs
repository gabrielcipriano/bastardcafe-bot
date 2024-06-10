use serde::Deserialize;
use teloxide::utils::markdown;

#[derive(Deserialize, Debug, Clone)]
pub struct Location {
    pub store: String,
    pub locale: String
}

impl Location {
    pub fn human_friendly(&self) -> String {
        format!("{store} - {locale}", store=self.store, locale=self.locale,)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BoardGame {
    pub url: String,
    pub name: String,
    pub locations: Vec<Location>,
    // pub raw: String,
}

impl BoardGame {
//     pub fn human_friendly(&self) -> String {
//         format!("{name}
// {locations}
// {url}", 
//         name=self.name,
//         locations=self.locations.iter().map(|loc| loc.human_friendly()).collect::<Vec<String>>().join("\n"),
//         url=self.url)
//     }

    pub fn markdown_v2(&self) -> String {
        format!("[{name}]({url}) \n{locations}", 
        name=markdown::escape(self.name.as_str()),
        locations=markdown::escape(self.locations.iter().map(|loc| loc.human_friendly()).collect::<Vec<String>>().join("\n").as_str()),
        url=markdown::escape(self.url.as_str()))
    }
}
