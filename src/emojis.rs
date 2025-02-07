use std::{
    fs::{read_dir, read_to_string},
    io::Result,
    path::PathBuf,
};

use aho_corasick::{AhoCorasick, MatchKind};

pub struct EmojiParser {
    svg_directory: PathBuf,
    aho_corasick: AhoCorasick,
}

impl EmojiParser {
    pub fn new(svg_directory: PathBuf) -> Result<Self> {
        let pattern = read_dir(&svg_directory)?
            .filter_map(|entry| {
                let file_name = entry.ok()?.file_name();
                let character_pattern = file_name
                    .to_string_lossy()
                    .strip_suffix(".svg")?
                    .split('-')
                    .filter_map(|code_point| {
                        u32::from_str_radix(code_point, 16)
                            .ok()
                            .and_then(char::from_u32)
                    })
                    .collect::<String>();

                Some(character_pattern)
            })
            .collect::<Vec<_>>();

        Ok(Self {
            svg_directory,
            aho_corasick: AhoCorasick::builder()
                .match_kind(MatchKind::LeftmostLongest)
                .build(&pattern)
                .unwrap(),
        })
    }

    pub fn inline_from_directory(&self, haystack: &str) -> String {
        let mut inlined_content = String::with_capacity(haystack.len());
        self.aho_corasick.replace_all_with(
            haystack,
            &mut inlined_content,
            |_, code_point_str, destination| {
                destination.push_str(&self.code_point_to_svg_tag(code_point_str.chars()));
                true
            },
        );
        inlined_content
    }

    fn code_point_to_svg_tag(&self, code_point: impl Iterator<Item = char> + Clone) -> String {
        let file_path = self
            .svg_directory
            .to_path_buf()
            .join(
                code_point
                    .clone()
                    .map(|c| format!("{:x}", c as u32))
                    .collect::<Vec<_>>()
                    .join("-"),
            )
            .with_extension("svg");

        let svg = read_to_string(file_path)
            .unwrap()
            .lines()
            .filter(|line| {
                let line_trim = line.trim_start();
                !line_trim.starts_with("<?xml") && !line_trim.starts_with("<!--")
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "<svg class=\"emoji\" draggable=\"false\" style=\"height: 1em; width: 1em; margin: 0 .05em 0 .1em; vertical-align: -0.1em;\" alt=\"{}\"{}",
            code_point.collect::<String>(),
            &svg[4..]
        )
    }
}
