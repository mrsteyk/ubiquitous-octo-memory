// could've been done with regex and stuff but meh, wasm is bad when it comes to memory management
// on the side note - idk if any crate can parse an array of KV thingies

// pub type Entity<'a> = hashbrown::HashMap<&'a str, &'a str>;

// pub fn parse_ents<'a>(data: &'a str) -> Vec<Entity<'a>> {
//     let mut ret = Vec::<Entity>::new();

//     let mut lines = data.lines();
//     loop {
//         if let Some(start) = lines.next() {
//             if start == "{" {
//                 let mut ent = Entity::new();
//                 loop {
//                     if let Some(data) = lines.next() {
//                         if data == "}" {
//                             break
//                         } else {
//                             // parse `"K" "V"`
//                         }
//                     } else {
//                         break
//                     }
//                 };
//                 ret.push(ent)
//             } else {
//                 continue;
//             }
//         } else {
//             break;
//         }
//     }

//     ret
// }

#[derive(Debug)]
pub struct Entity {
    pub string: String,
    pub dirty: bool,
}

const CLASSNAME_PREFIX: &str = "\"classname\" \"";
const ORIGIN_PREFIX: &str = "\"origin\" \"";

impl Entity {
    // TODO: idk why I did that if there's fmt::Display
    pub fn pretty_name(&self) -> String {
        let (classname, origin) = {
            let mut classname = String::new();
            let mut origin: Option<String> = None;
            for line in self.string.lines() {
                if line.starts_with(CLASSNAME_PREFIX) {
                    classname = line[CLASSNAME_PREFIX.len()..line.len() - 1].to_string();
                } else if line.starts_with(ORIGIN_PREFIX) {
                    origin = Some(line[CLASSNAME_PREFIX.len()..line.len() - 1].to_string());
                }
            }

            (classname, origin)
        };

        if let Some(origin) = origin {
            format!("{}<{}>", classname, origin)
        } else {
            format!("{}", classname)
        }
    }
}

// TODO: deprecate?
pub fn parse_ents_hacky(data: &str) -> Vec<Entity> {
    // let mut ret = Vec::new();
    data.split_inclusive("}\n")
        .map(|f| {
            let s = if f.len() > 3 {
                &f[0..f.len() - 1] // strip '\n'
            } else {
                f // a?
            };

            Entity {
                string: s.to_string(),
                dirty: false,
            }
        })
        .collect()
}
