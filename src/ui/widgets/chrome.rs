use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

use crate::ui::theme::ThemeTokens;
use crate::ui::widgets::parse_color;

pub fn card_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.foreground))
}

pub fn layered_panel_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.foreground))
}

pub fn raised_panel_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.foreground))
}

pub fn surface_style(theme: &ThemeTokens) -> Style {
    layered_panel_style(theme)
}

pub fn muted_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.shell_muted))
}

pub fn label_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.shell_title))
}

pub fn accent_style(theme: &ThemeTokens) -> Style {
    Style::default()
        .fg(parse_color(&theme.accent))
        .add_modifier(Modifier::BOLD)
}

pub fn separator_style(theme: &ThemeTokens) -> Style {
    Style::default().fg(parse_color(&theme.shell_separator))
}

pub fn selected_row_style(theme: &ThemeTokens) -> Style {
    Style::default()
        .fg(parse_color(&theme.shell_row_selected_fg))
        .bg(parse_color(&theme.shell_row_selected_bg))
        .add_modifier(Modifier::BOLD)
}

pub fn pill_style(theme: &ThemeTokens) -> Style {
    Style::default()
        .fg(parse_color(&theme.shell_title))
        .bg(parse_color(&theme.shell_pill_bg))
        .add_modifier(Modifier::BOLD)
}

pub fn pill<'a>(text: impl Into<Cow<'a, str>>, theme: &ThemeTokens) -> Span<'a> {
    let text: Cow<'a, str> = text.into();
    Span::styled(format!(" {} ", text), pill_style(theme))
}

pub fn compact_row<'a>(
    label: impl Into<Cow<'a, str>>,
    value: impl Into<Cow<'a, str>>,
    theme: &ThemeTokens,
    selected: bool,
) -> Line<'a> {
    let label: Cow<'a, str> = label.into();
    let value: Cow<'a, str> = value.into();
    let label_style = if selected {
        selected_row_style(theme)
    } else {
        Style::default().fg(parse_color(&theme.foreground))
    };
    let separator = if selected {
        Span::styled("›", separator_style(theme))
    } else {
        Span::styled(" ", separator_style(theme))
    };

    Line::from(vec![
        separator,
        Span::raw(" "),
        Span::styled(label, label_style),
        Span::raw("  "),
        Span::styled(value, label_style),
    ])
}

pub fn top_status_bar<'a, T>(lines: T, theme: &ThemeTokens) -> Paragraph<'a>
where
    T: Into<Text<'a>>,
{
    Paragraph::new(lines)
        .style(Style::default().fg(parse_color(&theme.shell_top_bar_fg)))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .style(Style::default().fg(parse_color(&theme.shell_separator)))
                .border_style(separator_style(theme)),
        )
}

pub fn bottom_shortcut_bar<'a, T>(lines: T, theme: &ThemeTokens) -> Paragraph<'a>
where
    T: Into<Text<'a>>,
{
    Paragraph::new(lines)
        .style(Style::default().fg(parse_color(&theme.shell_footer_fg)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .style(Style::default().fg(parse_color(&theme.shell_separator)))
                .border_style(separator_style(theme)),
        )
}

pub fn block_card(title: impl Into<Line<'static>>, theme: &ThemeTokens) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(card_style(theme))
        .border_style(separator_style(theme))
}

pub fn surface_block(title: impl Into<Line<'static>>, theme: &ThemeTokens) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::TOP)
        .style(surface_style(theme))
        .border_style(separator_style(theme))
}

pub fn footer_block(theme: &ThemeTokens) -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .style(Style::default().fg(parse_color(&theme.shell_footer_fg)))
        .border_style(separator_style(theme))
}

pub fn render_metric(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    label: &str,
    value: impl Into<String>,
    theme: &ThemeTokens,
) {
    let lines = vec![
        Line::from(Span::styled(
            label,
            label_style(theme).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            value.into(),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let widget = Paragraph::new(lines).block(surface_block("", theme));
    frame.render_widget(widget, area);
}

pub fn render_metric_card(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    label: &str,
    value: impl Into<String>,
    theme: &ThemeTokens,
) {
    let lines = vec![
        Line::from(Span::styled(
            label,
            label_style(theme).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            value.into(),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let widget = Paragraph::new(lines).block(block_card("", theme));
    frame.render_widget(widget, area);
}

pub struct CoverArtWidget<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub tag: &'a str,
    pub theme: &'a ThemeTokens,
    pub emphasis: Color,
}

impl Widget for CoverArtWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 12 || area.height < 6 {
            return;
        }

        let base = parse_color(&self.theme.shell_panel_raised);
        let text = parse_color(&self.theme.foreground);
        let accent = self.emphasis;
        let shadow = blend(base, parse_color(&self.theme.background), 0.45);
        let stripe = blend(accent, base, 0.58);
        let glow = blend(text, accent, 0.52);
        let seed = hash_seed(self.title);

        let inner = area;

        for y in inner.y..inner.bottom() {
            for x in inner.x..inner.right() {
                let vertical = (y - inner.y) as f32 / inner.height.max(1) as f32;
                let horizontal = ((x - inner.x) as u32 + seed) % 17;
                let bg = if vertical < 0.14 {
                    stripe
                } else if vertical < 0.58 {
                    shadow
                } else {
                    base
                };
                let (symbol, fg) = if horizontal == 0 || horizontal == 9 {
                    ("·", glow)
                } else if horizontal == 4 && vertical > 0.18 {
                    ("▁", blend(glow, stripe, 0.25))
                } else {
                    (" ", bg)
                };
                buf.set_string(x, y, symbol, Style::default().fg(fg).bg(bg));
            }
        }

        let initials = cover_initials(self.title);
        let center_y = inner.y + inner.height / 2;
        let title_y = center_y.saturating_sub(3);
        let initial_style = Style::default()
            .fg(glow)
            .bg(base)
            .add_modifier(Modifier::BOLD);

        if inner.width > 4 {
            let initials_width = initials.chars().count() as u16;
            let x = inner.x + inner.width.saturating_sub(initials_width) / 2;
            buf.set_string(x, title_y, initials, initial_style);
        }

        let subtitle = Paragraph::new(vec![
            Line::from(Span::styled(
                self.tag,
                Style::default()
                    .fg(accent)
                    .bg(base)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(self.title, Style::default().fg(text).bg(base))),
            Line::from(Span::styled(
                self.subtitle,
                muted_style(self.theme).bg(base),
            )),
        ])
        .alignment(Alignment::Left);

        subtitle.render(
            Rect {
                x: inner.x + 2,
                y: inner.bottom().saturating_sub(5),
                width: inner.width.saturating_sub(4),
                height: 4,
            },
            buf,
        );
    }
}

pub fn cover_initials(title: &str) -> String {
    let mut letters = title
        .split_whitespace()
        .filter_map(|part| part.chars().find(|c| c.is_ascii_alphanumeric()))
        .take(2)
        .map(|c| c.to_ascii_uppercase())
        .collect::<String>();
    if letters.is_empty() {
        letters.push('C');
    }
    letters
}

pub fn blend(a: Color, b: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);

    Color::Rgb(
        mix_channel(ar, br, amount),
        mix_channel(ag, bg, amount),
        mix_channel(ab, bb, amount),
    )
}

fn mix_channel(a: u8, b: u8, amount: f32) -> u8 {
    ((a as f32 * (1.0 - amount)) + (b as f32 * amount)).round() as u8
}

fn rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        Color::Gray => (127, 127, 127),
        Color::DarkGray => (64, 64, 64),
        Color::Red => (255, 0, 0),
        Color::Green => (0, 255, 0),
        Color::Blue => (0, 0, 255),
        Color::Yellow => (255, 255, 0),
        Color::Magenta => (255, 0, 255),
        Color::Cyan => (0, 255, 255),
        _ => (180, 180, 180),
    }
}

fn hash_seed(value: &str) -> u32 {
    value.bytes().fold(2166136261, |hash, byte| {
        hash.wrapping_mul(16777619) ^ u32::from(byte)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        bottom_shortcut_bar, card_style, compact_row, cover_initials, layered_panel_style, pill,
        pill_style, raised_panel_style, selected_row_style, separator_style, top_status_bar,
    };
    use crate::ui::theme::ThemeTokens;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::style::{Color, Modifier};
    use ratatui::text::Line;

    fn theme() -> ThemeTokens {
        ThemeTokens {
            name: "test".to_string(),
            background: "#101010".to_string(),
            foreground: "#f5f5f5".to_string(),
            accent: "#55ccff".to_string(),
            shell_top_bar_bg: "#202020".to_string(),
            shell_top_bar_fg: "#fafafa".to_string(),
            shell_footer_bg: "#181818".to_string(),
            shell_footer_fg: "#ededed".to_string(),
            shell_panel: "#262626".to_string(),
            shell_panel_alt: "#303030".to_string(),
            shell_panel_raised: "#343434".to_string(),
            shell_row_selected_bg: "#55ccff".to_string(),
            shell_row_selected_fg: "#101010".to_string(),
            shell_separator: "#666666".to_string(),
            shell_border: "#888888".to_string(),
            shell_pill_bg: "#223344".to_string(),
            shell_title: "#ffcc88".to_string(),
            shell_success: "#a0c080".to_string(),
            shell_warning: "#d8a060".to_string(),
            shell_muted: "#808080".to_string(),
            shell_error: "#d06070".to_string(),
            lane_border: "#888888".to_string(),
            lane_fill: "#262626".to_string(),
            lane_active: "#55ccff".to_string(),
            hud_panel: "#303030".to_string(),
            hud_label: "#ffcc88".to_string(),
            judgment_perfect: "#a0c080".to_string(),
            judgment_great: "#55ccff".to_string(),
            judgment_good: "#d8a060".to_string(),
            judgment_miss: "#d06070".to_string(),
        }
    }

    #[test]
    fn cover_initials_keeps_existing_focus_art_behavior() {
        assert_eq!(cover_initials("Demo Hold"), "DH");
        assert_eq!(cover_initials("night transit"), "NT");
    }

    #[test]
    fn shell_selected_rows_use_the_selected_palette() {
        let style = selected_row_style(&theme());

        assert_eq!(style.fg, Some(Color::Rgb(0x10, 0x10, 0x10)));
        assert_eq!(style.bg, Some(Color::Rgb(0x55, 0xcc, 0xff)));
    }

    #[test]
    fn shell_pills_render_with_the_pill_background() {
        let token = pill("TAG", &theme());

        assert_eq!(token.content, " TAG ");
        assert_eq!(token.style, pill_style(&theme()));
    }

    #[test]
    fn shell_compact_rows_style_selected_and_unselected_entries() {
        let selected = compact_row("Song", "Demo Hold", &theme(), true);
        let unselected = compact_row("Song", "Demo Hold", &theme(), false);

        assert_eq!(selected.spans[0].content, "›");
        assert_eq!(selected.spans[0].style.fg, separator_style(&theme()).fg);
        assert_eq!(selected.spans[2].content, "Song");
        assert_eq!(selected.spans[2].style, selected_row_style(&theme()));
        assert_eq!(selected.spans[4].style, selected_row_style(&theme()));

        assert_eq!(unselected.spans[0].style.fg, separator_style(&theme()).fg);
        assert_eq!(
            unselected.spans[2].style.fg,
            Some(Color::Rgb(0xf5, 0xf5, 0xf5))
        );
        assert_eq!(
            unselected.spans[4].style.fg,
            Some(Color::Rgb(0xf5, 0xf5, 0xf5))
        );
        assert_eq!(unselected.spans[2].style.add_modifier, Modifier::empty());
    }

    #[test]
    fn shell_card_styles_draw_from_shell_panels() {
        let style = card_style(&theme());
        let layered = layered_panel_style(&theme());
        let raised = raised_panel_style(&theme());

        assert_eq!(style.bg, None);
        assert_eq!(layered.bg, None);
        assert_eq!(raised.bg, None);
    }

    #[test]
    fn shell_separator_style_uses_the_separator_token() {
        let style = separator_style(&theme());

        assert_eq!(style.fg, Some(Color::Rgb(0x66, 0x66, 0x66)));
    }

    #[test]
    fn top_status_bar_uses_shell_top_bar_tokens_and_separator_border() {
        let theme = theme();
        let widget = top_status_bar(vec![Line::from("status")], &theme);
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 3));

        widget.render(Rect::new(0, 0, 12, 3), &mut buf);

        let top_left = buf.cell((0, 0)).unwrap();
        let border = buf.cell((0, 2)).unwrap();

        assert_eq!(top_left.fg, Color::Rgb(0xfa, 0xfa, 0xfa));
        assert_eq!(top_left.bg, Color::Reset);
        assert_eq!(border.fg, Color::Rgb(0x66, 0x66, 0x66));
        assert_ne!(border.symbol(), " ");
    }

    #[test]
    fn bottom_shortcut_bar_uses_shell_footer_tokens_and_separator_border() {
        let theme = theme();
        let widget = bottom_shortcut_bar(vec![Line::from("shortcuts")], &theme);
        let mut buf = Buffer::empty(Rect::new(0, 0, 14, 3));

        widget.render(Rect::new(0, 0, 14, 3), &mut buf);

        let top_border = buf.cell((0, 0)).unwrap();
        let bottom_row = buf.cell((0, 2)).unwrap();

        assert_eq!(top_border.fg, Color::Rgb(0x66, 0x66, 0x66));
        assert_ne!(top_border.symbol(), " ");
        assert_eq!(bottom_row.fg, Color::Rgb(0xed, 0xed, 0xed));
        assert_eq!(bottom_row.bg, Color::Reset);
    }
}
