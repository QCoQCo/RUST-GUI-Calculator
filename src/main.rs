use iced::{
    Application, Command, Element, Length, Settings, Theme,
    widget::{button, column, container, row, text},
    Background, Color,
};

fn main() -> iced::Result {
    Calculator::run(Settings {
        window: iced::window::Settings {
            size: (500, 600),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Message {
    // 버튼 클릭 메시지 (3단계에서 구현 예정)
    ButtonPressed(String),
}

struct Calculator {
    display: String, // 디스플레이에 표시할 값
}

impl Application for Calculator {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Calculator {
                display: "0".to_string(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Calculator")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ButtonPressed(_) => {
                // TODO: 4단계에서 계산 로직 구현
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // 메인 컬럼: 디스플레이 + 버튼 그리드
        column![
            // 디스플레이 영역
            display_area(&self.display),
            // 버튼 그리드
            button_grid()
        ]
        .spacing(10)
        .padding(20)
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// 디스플레이 영역 컴포넌트
fn display_area(value: &str) -> Element<'_, Message> {
    struct DisplayStyle;

    impl container::StyleSheet for DisplayStyle {
        type Style = Theme;

        fn appearance(&self, _style: &Self::Style) -> container::Appearance {
            container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
                border_color: Color::from_rgb(0.3, 0.3, 0.3),
                border_width: 2.0,
                border_radius: 5.0.into(),
                ..Default::default()
            }
        }
    }

    container(
        text(value)
            .size(48)
            .width(Length::Fill)
            .horizontal_alignment(iced::alignment::Horizontal::Right),
    )
    .width(Length::Fill)
    .height(Length::Fixed(100.0))
    .padding(20)
    .style(iced::theme::Container::Custom(Box::new(DisplayStyle)))
    .into()
}

// 버튼 그리드 레이아웃
fn button_grid() -> Element<'static, Message> {
    column![
        // 첫 번째 행: C, ±, %, ÷
        row![
            calc_button("C", "clear"),
            calc_button("±", "sign"),
            calc_button("%", "percent"),
            calc_button("÷", "divide"),
        ]
        .spacing(10),
        // 두 번째 행: 7, 8, 9, ×
        row![
            calc_button("7", "7"),
            calc_button("8", "8"),
            calc_button("9", "9"),
            calc_button("×", "multiply"),
        ]
        .spacing(10),
        // 세 번째 행: 4, 5, 6, -
        row![
            calc_button("4", "4"),
            calc_button("5", "5"),
            calc_button("6", "6"),
            calc_button("-", "subtract"),
        ]
        .spacing(10),
        // 네 번째 행: 1, 2, 3, +
        row![
            calc_button("1", "1"),
            calc_button("2", "2"),
            calc_button("3", "3"),
            calc_button("+", "add"),
        ]
        .spacing(10),
        // 다섯 번째 행: 0 (넓게), ., =
        row![
            calc_button("0", "0").width(Length::Fill),
            calc_button(".", "decimal"),
            calc_button("=", "equals"),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .into()
}

// 계산기 버튼 생성 함수
fn calc_button<'a>(label: &'a str, value: &'a str) -> button::Button<'a, Message> {
    button(text(label).size(24))
        .width(Length::Fixed(100.0))
        .height(Length::Fixed(70.0))
        .on_press(Message::ButtonPressed(value.to_string()))
}
