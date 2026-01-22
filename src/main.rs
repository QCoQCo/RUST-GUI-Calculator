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

// 연산자 타입 정의
#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    Add,      // +
    Subtract, // -
    Multiply, // ×
    Divide,   // ÷
}

// 버튼 타입 정의
#[derive(Debug, Clone)]
enum ButtonType {
    Number(char),        // 0-9
    Decimal,             // .
    Operator(Operator),  // +, -, ×, ÷
    Equals,              // =
    Clear,               // C
    Sign,                // ±
    Percent,             // %
}

#[derive(Debug, Clone)]
enum Message {
    ButtonPressed(ButtonType),
}

struct Calculator {
    display: String,              // 현재 표시할 값
    previous_value: Option<f64>,  // 이전 값
    operator: Option<Operator>,  // 현재 연산자
    waiting_for_operand: bool,    // 새 피연산자 입력 대기 중인지
    has_decimal: bool,           // 현재 입력값에 소수점이 있는지
}

impl Calculator {
    // 상태 초기화
    fn clear(&mut self) {
        self.display = "0".to_string();
        self.previous_value = None;
        self.operator = None;
        self.waiting_for_operand = false;
        self.has_decimal = false;
    }

    // 디스플레이 값을 숫자로 변환
    fn get_display_value(&self) -> f64 {
        self.display.parse().unwrap_or(0.0)
    }

    // 디스플레이 업데이트 (숫자 포맷팅)
    fn update_display(&mut self, value: f64) {
        // 무한대나 NaN 체크
        if value.is_infinite() || value.is_nan() {
            self.display = "Error".to_string();
            return;
        }

        // 소수점이 없고 정수인 경우 정수로 표시
        if value.fract() == 0.0 {
            self.display = value.trunc().to_string();
        } else {
            // 소수점이 있는 경우 최대 10자리까지 표시
            self.display = format!("{:.10}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
        }
    }

    // 숫자 입력 처리
    fn input_number(&mut self, digit: char) {
        if self.waiting_for_operand {
            self.display = digit.to_string();
            self.waiting_for_operand = false;
            self.has_decimal = false;
        } else {
            if self.display == "0" {
                self.display = digit.to_string();
            } else {
                self.display.push(digit);
            }
        }
    }

    // 소수점 입력 처리
    fn input_decimal(&mut self) {
        if self.waiting_for_operand {
            self.display = "0.".to_string();
            self.waiting_for_operand = false;
            self.has_decimal = true;
        } else if !self.has_decimal {
            self.display.push('.');
            self.has_decimal = true;
        }
    }

    // 연산자 입력 처리
    fn input_operator(&mut self, op: Operator) {
        let current_value = self.get_display_value();

        if let Some(prev_op) = self.operator {
            // 이전 연산자가 있으면 먼저 계산 실행
            if let Some(prev_val) = self.previous_value {
                let result = self.calculate(prev_val, prev_op, current_value);
                self.update_display(result);
                self.previous_value = Some(result);
            }
        } else {
            // 이전 연산자가 없으면 현재 값을 이전 값으로 저장
            self.previous_value = Some(current_value);
        }

        self.operator = Some(op);
        self.waiting_for_operand = true;
        self.has_decimal = false;
    }

    // 계산 실행
    fn calculate(&self, left: f64, op: Operator, right: f64) -> f64 {
        match op {
            Operator::Add => left + right,
            Operator::Subtract => left - right,
            Operator::Multiply => left * right,
            Operator::Divide => {
                if right == 0.0 {
                    f64::INFINITY // 0으로 나누기 에러
                } else {
                    left / right
                }
            }
        }
    }

    // 계산 실행 (= 버튼)
    fn execute_calculation(&mut self) {
        if let Some(op) = self.operator {
            if let Some(prev_val) = self.previous_value {
                let current_value = self.get_display_value();
                let result = self.calculate(prev_val, op, current_value);
                self.update_display(result);
                self.previous_value = None;
                self.operator = None;
                self.waiting_for_operand = true;
                self.has_decimal = result.fract() != 0.0;
            }
        }
    }

    // 부호 변경 (±)
    fn toggle_sign(&mut self) {
        let value = self.get_display_value();
        if value != 0.0 {
            self.update_display(-value);
        }
    }

    // 퍼센트 계산 (%)
    fn calculate_percent(&mut self) {
        let value = self.get_display_value();
        self.update_display(value / 100.0);
        self.has_decimal = true;
    }
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
                previous_value: None,
                operator: None,
                waiting_for_operand: false,
                has_decimal: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Calculator")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ButtonPressed(button_type) => {
                match button_type {
                    ButtonType::Clear => {
                        self.clear();
                    }
                    ButtonType::Number(digit) => {
                        self.input_number(digit);
                    }
                    ButtonType::Decimal => {
                        self.input_decimal();
                    }
                    ButtonType::Operator(op) => {
                        self.input_operator(op);
                    }
                    ButtonType::Equals => {
                        self.execute_calculation();
                    }
                    ButtonType::Sign => {
                        self.toggle_sign();
                    }
                    ButtonType::Percent => {
                        self.calculate_percent();
                    }
                }
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
            calc_button("C", ButtonType::Clear),
            calc_button("±", ButtonType::Sign),
            calc_button("%", ButtonType::Percent),
            calc_button("÷", ButtonType::Operator(Operator::Divide)),
        ]
        .spacing(10),
        // 두 번째 행: 7, 8, 9, ×
        row![
            calc_button("7", ButtonType::Number('7')),
            calc_button("8", ButtonType::Number('8')),
            calc_button("9", ButtonType::Number('9')),
            calc_button("×", ButtonType::Operator(Operator::Multiply)),
        ]
        .spacing(10),
        // 세 번째 행: 4, 5, 6, -
        row![
            calc_button("4", ButtonType::Number('4')),
            calc_button("5", ButtonType::Number('5')),
            calc_button("6", ButtonType::Number('6')),
            calc_button("-", ButtonType::Operator(Operator::Subtract)),
        ]
        .spacing(10),
        // 네 번째 행: 1, 2, 3, +
        row![
            calc_button("1", ButtonType::Number('1')),
            calc_button("2", ButtonType::Number('2')),
            calc_button("3", ButtonType::Number('3')),
            calc_button("+", ButtonType::Operator(Operator::Add)),
        ]
        .spacing(10),
        // 다섯 번째 행: 0 (넓게), ., =
        row![
            calc_button("0", ButtonType::Number('0')).width(Length::Fill),
            calc_button(".", ButtonType::Decimal),
            calc_button("=", ButtonType::Equals),
        ]
        .spacing(10),
    ]
    .spacing(10)
    .into()
}

// 계산기 버튼 생성 함수
fn calc_button<'a>(label: &'a str, button_type: ButtonType) -> button::Button<'a, Message> {
    button(text(label).size(24))
        .width(Length::Fixed(100.0))
        .height(Length::Fixed(70.0))
        .on_press(Message::ButtonPressed(button_type))
}
