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

// 입력 가능한 최대 자릿수 (f64 정밀도 한계)
const MAX_INPUT_DIGITS: usize = 15;

// 이 범위를 벗어나는 값은 지수 표기로 표시
const SCIENTIFIC_UPPER: f64 = 1e15;
const SCIENTIFIC_LOWER: f64 = 1e-7;

// 레이아웃 크기
const BUTTON_WIDTH: f32 = 100.0;
const BUTTON_HEIGHT: f32 = 75.0;
const SPACING: u16 = 10;
const GRID_WIDTH: f32 = BUTTON_WIDTH * 4.0 + SPACING as f32 * 3.0;

struct Calculator {
    display: String,               // 현재 표시할 값
    previous_value: Option<f64>,   // 이전 값
    operator: Option<Operator>,    // 현재 연산자
    waiting_for_operand: bool,     // 다음 숫자 입력 시 새로 시작하는지
    operator_just_pressed: bool,   // 직전 입력이 연산자였는지
    error: bool,                   // 에러 상태인지 (0으로 나누기, 오버플로)
}

impl Calculator {
    // 상태 초기화
    fn clear(&mut self) {
        self.display = "0".to_string();
        self.previous_value = None;
        self.operator = None;
        self.waiting_for_operand = false;
        self.operator_just_pressed = false;
        self.error = false;
    }

    // 화면에 보여줄 문자열
    fn display_text(&self) -> &str {
        if self.error {
            "Error"
        } else {
            &self.display
        }
    }

    // 디스플레이 값을 숫자로 변환
    fn get_display_value(&self) -> f64 {
        self.display.parse().unwrap_or(0.0)
    }

    // 디스플레이 업데이트 (숫자 포맷팅)
    fn update_display(&mut self, value: f64) {
        self.display = format_number(value);
    }

    // 숫자 입력 처리
    fn input_number(&mut self, digit: char) {
        // Error 상태이면 초기화
        if self.error {
            self.clear();
        }
        
        if self.waiting_for_operand {
            self.display = digit.to_string();
            self.waiting_for_operand = false;
        } else if self.display == "0" {
            self.display = digit.to_string();
        } else if count_digits(&self.display) < MAX_INPUT_DIGITS {
            self.display.push(digit);
        }
    }

    // 소수점 입력 처리
    fn input_decimal(&mut self) {
        // Error 상태이면 초기화
        if self.error {
            self.clear();
        }
        
        if self.waiting_for_operand {
            self.display = "0.".to_string();
            self.waiting_for_operand = false;
        } else if !self.display.contains('.') {
            self.display.push('.');
        }
    }

    // 연산자 입력 처리
    fn input_operator(&mut self, op: Operator) {
        // Error 상태이면 연산자 입력 무시
        if self.error {
            return;
        }

        // 연산자를 연속으로 누르면 연산자만 교체
        if self.operator_just_pressed && self.operator.is_some() {
            self.operator = Some(op);
            return;
        }
        
        let current_value = self.get_display_value();

        if let Some(prev_op) = self.operator {
            // 이전 연산자가 있으면 먼저 계산 실행
            if let Some(prev_val) = self.previous_value {
                // Error가 발생하면 연산자 설정하지 않음
                let Some(result) = calculate(prev_val, prev_op, current_value) else {
                    self.error = true;
                    return;
                };
                self.update_display(result);
                self.previous_value = Some(result);
            }
        } else {
            // 이전 연산자가 없으면 현재 값을 이전 값으로 저장
            self.previous_value = Some(current_value);
        }

        self.operator = Some(op);
        self.waiting_for_operand = true;
    }

    // 계산 실행 (= 버튼)
    fn execute_calculation(&mut self) {
        // Error 상태이면 계산 실행 무시
        if self.error {
            return;
        }
        
        if let (Some(op), Some(prev_val)) = (self.operator, self.previous_value) {
            let current_value = self.get_display_value();
            match calculate(prev_val, op, current_value) {
                Some(result) => {
                    self.update_display(result);
                    self.previous_value = None;
                    self.operator = None;
                    self.waiting_for_operand = true;
                }
                None => self.error = true,
            }
        }
    }

    // 부호 변경 (±)
    fn toggle_sign(&mut self) {
        // Error 상태이면 무시
        if self.error {
            return;
        }
        
        // 문자열에서 부호만 바꿔 입력 중인 형태(예: "1.")를 유지
        if let Some(positive) = self.display.strip_prefix('-') {
            self.display = positive.to_string();
        } else if self.display != "0" {
            self.display.insert(0, '-');
        }
    }

    // 퍼센트 계산 (%)
    fn calculate_percent(&mut self) {
        // Error 상태이면 무시
        if self.error {
            return;
        }
        
        let value = self.get_display_value();
        self.update_display(value / 100.0);
        // 결과 뒤 숫자 입력은 새 입력으로 시작
        self.waiting_for_operand = true;
    }
}

// 계산 실행 (0으로 나누기, 오버플로 시 None)
fn calculate(left: f64, op: Operator, right: f64) -> Option<f64> {
    let result = match op {
        Operator::Add => left + right,
        Operator::Subtract => left - right,
        Operator::Multiply => left * right,
        Operator::Divide => {
            if right == 0.0 {
                return None;
            }
            left / right
        }
    };
    Some(result).filter(|r| r.is_finite())
}

// 숫자를 디스플레이용 문자열로 변환 (유한한 값만 받음)
fn format_number(value: f64) -> String {
    debug_assert!(value.is_finite(), "유한하지 않은 값: {value}");

    // -0.0도 "0"으로 표시
    if value == 0.0 {
        return "0".to_string();
    }

    let abs = value.abs();
    if abs >= SCIENTIFIC_UPPER || abs < SCIENTIFIC_LOWER {
        // 지수 표기: 가수 최대 9자리, 끝의 0 제거 (예: 9.9999997e23)
        let formatted = format!("{:.8e}", value);
        let (mantissa, exponent) = formatted
            .split_once('e')
            .expect("지수 표기에는 항상 'e'가 있음");
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        format!("{mantissa}e{exponent}")
    } else if value.fract() == 0.0 {
        // 정수인 경우 정수로 표시
        value.to_string()
    } else {
        // 소수점이 있는 경우 최대 10자리까지 표시
        format!("{:.10}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// 문자열에 포함된 숫자 자릿수 (부호, 소수점 제외)
fn count_digits(s: &str) -> usize {
    s.chars().filter(|c| c.is_ascii_digit()).count()
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
                operator_just_pressed: false,
                error: false,
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
                let is_operator = matches!(button_type, ButtonType::Operator(_));
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
                self.operator_just_pressed = is_operator;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // 메인 컬럼: 디스플레이 + 버튼 그리드
        let content = column![
            // 디스플레이 영역
            display_area(self.display_text()),
            // 버튼 그리드
            button_grid()
        ]
        .spacing(SPACING);

        // 창 크기가 바뀌어도 계산기를 가운데에 배치
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

// 디스플레이 영역 컴포넌트 (개선: 긴 숫자 처리)
fn display_area(value: &str) -> Element<'_, Message> {
    struct DisplayStyle;

    impl container::StyleSheet for DisplayStyle {
        type Style = Theme;

        fn appearance(&self, _style: &Self::Style) -> container::Appearance {
            container::Appearance {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                border_color: Color::from_rgb(0.4, 0.4, 0.4),
                border_width: 2.0,
                border_radius: 10.0.into(),
                ..Default::default()
            }
        }
    }

    // 긴 숫자에 대해 폰트 크기 자동 조정
    let font_size = if value.len() > 12 {
        32.0
    } else if value.len() > 8 {
        40.0
    } else {
        48.0
    };

    container(
        text(value)
            .size(font_size as u16)
            .width(Length::Fill)
            .horizontal_alignment(iced::alignment::Horizontal::Right),
    )
    .width(Length::Fixed(GRID_WIDTH))
    .height(Length::Fixed(120.0))
    .padding(20)
    .style(iced::theme::Container::Custom(Box::new(DisplayStyle)))
    .into()
}

// 버튼 스타일 타입
enum ButtonStyleType {
    Number,    // 숫자 버튼
    Operator,  // 연산자 버튼
    Function,  // 기능 버튼 (C, ±, %)
    Equals,    // = 버튼
}

// 버튼 스타일 시트
struct ButtonStyle {
    style_type: ButtonStyleType,
}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let (background, text_color) = match self.style_type {
            ButtonStyleType::Number => {
                // 숫자 버튼: 어두운 회색
                (Color::from_rgb(0.3, 0.3, 0.3), Color::WHITE)
            }
            ButtonStyleType::Operator => {
                // 연산자 버튼: 주황색
                (Color::from_rgb(1.0, 0.6, 0.0), Color::WHITE)
            }
            ButtonStyleType::Function => {
                // 기능 버튼: 밝은 회색
                (Color::from_rgb(0.5, 0.5, 0.5), Color::WHITE)
            }
            ButtonStyleType::Equals => {
                // = 버튼: 파란색
                (Color::from_rgb(0.0, 0.5, 1.0), Color::WHITE)
            }
        };

        button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border_radius: 10.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        // 호버 시 약간 밝게
        if let Some(Background::Color(color)) = appearance.background {
            appearance.background = Some(Background::Color(Color {
                r: (color.r + 0.1).min(1.0),
                g: (color.g + 0.1).min(1.0),
                b: (color.b + 0.1).min(1.0),
                a: color.a,
            }));
        }
        appearance
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        // 눌렀을 때 약간 어둡게
        if let Some(Background::Color(color)) = appearance.background {
            appearance.background = Some(Background::Color(Color {
                r: (color.r - 0.1).max(0.0),
                g: (color.g - 0.1).max(0.0),
                b: (color.b - 0.1).max(0.0),
                a: color.a,
            }));
        }
        appearance
    }
}

// 버튼 그리드 레이아웃
fn button_grid() -> Element<'static, Message> {
    column![
        // 첫 번째 행: C, ±, %, ÷
        row![
            calc_button("C", ButtonType::Clear, ButtonStyleType::Function),
            calc_button("±", ButtonType::Sign, ButtonStyleType::Function),
            calc_button("%", ButtonType::Percent, ButtonStyleType::Function),
            calc_button("÷", ButtonType::Operator(Operator::Divide), ButtonStyleType::Operator),
        ]
        .spacing(SPACING),
        // 두 번째 행: 7, 8, 9, ×
        row![
            calc_button("7", ButtonType::Number('7'), ButtonStyleType::Number),
            calc_button("8", ButtonType::Number('8'), ButtonStyleType::Number),
            calc_button("9", ButtonType::Number('9'), ButtonStyleType::Number),
            calc_button("×", ButtonType::Operator(Operator::Multiply), ButtonStyleType::Operator),
        ]
        .spacing(SPACING),
        // 세 번째 행: 4, 5, 6, -
        row![
            calc_button("4", ButtonType::Number('4'), ButtonStyleType::Number),
            calc_button("5", ButtonType::Number('5'), ButtonStyleType::Number),
            calc_button("6", ButtonType::Number('6'), ButtonStyleType::Number),
            calc_button("-", ButtonType::Operator(Operator::Subtract), ButtonStyleType::Operator),
        ]
        .spacing(SPACING),
        // 네 번째 행: 1, 2, 3, +
        row![
            calc_button("1", ButtonType::Number('1'), ButtonStyleType::Number),
            calc_button("2", ButtonType::Number('2'), ButtonStyleType::Number),
            calc_button("3", ButtonType::Number('3'), ButtonStyleType::Number),
            calc_button("+", ButtonType::Operator(Operator::Add), ButtonStyleType::Operator),
        ]
        .spacing(SPACING),
        // 다섯 번째 행: 0 (버튼 2칸 + 간격), ., =
        row![
            calc_button("0", ButtonType::Number('0'), ButtonStyleType::Number)
                .width(Length::Fixed(BUTTON_WIDTH * 2.0 + SPACING as f32)),
            calc_button(".", ButtonType::Decimal, ButtonStyleType::Number),
            calc_button("=", ButtonType::Equals, ButtonStyleType::Equals),
        ]
        .spacing(SPACING),
    ]
    .spacing(SPACING)
    .into()
}

// 계산기 버튼 생성 함수 (스타일 적용)
fn calc_button<'a>(
    label: &'a str,
    button_type: ButtonType,
    style_type: ButtonStyleType,
) -> button::Button<'a, Message> {
    // 라벨을 버튼 가운데에 배치
    let label = text(label)
        .size(28)
        .width(Length::Fill)
        .height(Length::Fill)
        .horizontal_alignment(iced::alignment::Horizontal::Center)
        .vertical_alignment(iced::alignment::Vertical::Center);

    button(label)
        .width(Length::Fixed(BUTTON_WIDTH))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(iced::theme::Button::Custom(Box::new(ButtonStyle {
            style_type,
        })))
        .on_press(Message::ButtonPressed(button_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_calc() -> Calculator {
        Calculator::new(()).0
    }

    // 키 문자열을 버튼 입력으로 변환해 순서대로 누름
    // 0-9 . + - * / = C ± %
    fn press(calc: &mut Calculator, keys: &str) {
        for key in keys.chars() {
            let button_type = match key {
                '0'..='9' => ButtonType::Number(key),
                '.' => ButtonType::Decimal,
                '+' => ButtonType::Operator(Operator::Add),
                '-' => ButtonType::Operator(Operator::Subtract),
                '*' => ButtonType::Operator(Operator::Multiply),
                '/' => ButtonType::Operator(Operator::Divide),
                '=' => ButtonType::Equals,
                'C' => ButtonType::Clear,
                '±' => ButtonType::Sign,
                '%' => ButtonType::Percent,
                _ => panic!("알 수 없는 키: {key}"),
            };
            let _ = calc.update(Message::ButtonPressed(button_type));
        }
    }

    fn display_after(keys: &str) -> String {
        let mut calc = new_calc();
        press(&mut calc, keys);
        calc.display_text().to_string()
    }

    // ===== 기존 동작 (회귀 방지) =====

    #[test]
    fn adds_two_numbers() {
        assert_eq!(display_after("12+34="), "46");
    }

    #[test]
    fn chains_operations_left_to_right() {
        assert_eq!(display_after("2+3*4="), "20");
    }

    #[test]
    fn hides_float_rounding_error() {
        assert_eq!(display_after("0.1+0.2="), "0.3");
    }

    #[test]
    fn divide_by_zero_shows_error() {
        assert_eq!(display_after("5/0="), "Error");
    }

    #[test]
    fn digit_after_error_starts_fresh() {
        assert_eq!(display_after("5/0=7"), "7");
        assert_eq!(display_after("5/0=7+1="), "8");
    }

    #[test]
    fn clear_resets_everything() {
        assert_eq!(display_after("12+3C"), "0");
        assert_eq!(display_after("12+3C4="), "4");
    }

    #[test]
    fn toggle_sign_on_result() {
        assert_eq!(display_after("2+3=±"), "-5");
    }

    #[test]
    fn percent_divides_by_hundred() {
        assert_eq!(display_after("50%"), "0.5");
    }

    // ===== 버그 재현 (doc/bugAndFix.md) =====

    // 버그 1: 연산자를 연속으로 누르면 연산자만 교체되어야 함
    #[test]
    fn bug1_consecutive_operators_replace_operator() {
        assert_eq!(display_after("5+*"), "5");
        assert_eq!(display_after("5+*3="), "15");
    }

    // 버그 2-A: ± 후에도 입력 중인 소수점이 유지되어야 함
    #[test]
    fn bug2a_toggle_sign_keeps_decimal_input() {
        assert_eq!(display_after("1.±"), "-1.");
        assert_eq!(display_after("1.±5"), "-1.5");
    }

    // 버그 2-B: % 결과가 정수여도 바로 소수점을 입력할 수 있어야 함
    // (버그 3 수정 방침에 따라 % 뒤 입력은 새 입력으로 시작)
    #[test]
    fn bug2b_decimal_allowed_after_integer_percent() {
        assert_eq!(display_after("500%"), "5");
        assert_eq!(display_after("500%.5"), "0.5");
    }

    // 버그 3: % 결과 뒤 숫자 입력은 새 입력으로 시작해야 함
    #[test]
    fn bug3_digit_after_percent_starts_new_input() {
        assert_eq!(display_after("50%3"), "3");
    }

    // 버그 4: -0이 표시되면 안 됨
    #[test]
    fn bug4_no_negative_zero() {
        assert_eq!(display_after("0*5±="), "0");
        assert_eq!(display_after("0/5±="), "0");
    }

    // 버그 5: 아주 크거나 작은 결과는 짧게(지수 표기로) 표시되어야 함
    fn assert_compact_and_close(display: &str, expected: f64) {
        assert!(
            display.len() <= 16,
            "표시가 너무 김 ({}자): {display}",
            display.len()
        );
        let value: f64 = display
            .parse()
            .unwrap_or_else(|_| panic!("숫자로 해석 불가: {display}"));
        let relative_error = ((value - expected) / expected).abs();
        assert!(
            relative_error < 1e-6,
            "값이 다름: {display} (기대값 {expected:e})"
        );
    }

    #[test]
    fn bug5_huge_result_is_compact() {
        let display = display_after("99999999*99999999*99999999=");
        assert_compact_and_close(&display, 99999999f64.powi(3));
    }

    #[test]
    fn bug5_tiny_result_is_not_rounded_to_zero() {
        let display = display_after("0.00000001/1000=");
        assert_compact_and_close(&display, 1e-11);
    }

    // 버그 6: 입력 자릿수는 최대 15자리로 제한되어야 함
    #[test]
    fn bug6_input_digits_are_limited() {
        let display = display_after(&"9".repeat(30));
        let digits = display.chars().filter(|c| c.is_ascii_digit()).count();
        assert_eq!(digits, 15, "입력된 자릿수: {display}");
    }

    #[test]
    fn bug6_digit_limit_excludes_sign_and_decimal() {
        let keys = format!("1.{}", "2".repeat(30));
        let display = display_after(&keys);
        let digits = display.chars().filter(|c| c.is_ascii_digit()).count();
        assert_eq!(digits, 15, "입력된 자릿수: {display}");
        assert!(display.starts_with("1."));
    }

    // ===== 수정 과정에서 추가한 경계 사례 =====

    // % 결과를 피연산자로 쓴 뒤 연산자를 눌러도 값이 버려지지 않아야 함
    #[test]
    fn percent_operand_is_kept_before_next_operator() {
        assert_eq!(display_after("200+10%+"), "200.1");
        assert_eq!(display_after("200+10%+5="), "205.1");
    }

    // = 결과에 이어서 연산자를 누르면 결과가 이전 값이 되어야 함
    #[test]
    fn operator_after_equals_uses_result() {
        assert_eq!(display_after("2+3=*4="), "20");
    }

    #[test]
    fn toggle_sign_on_zero_does_nothing() {
        assert_eq!(display_after("±"), "0");
    }

    #[test]
    fn toggle_sign_twice_restores_input() {
        assert_eq!(display_after("1.5±±"), "1.5");
    }

    #[test]
    fn decimal_input_still_allowed_once() {
        assert_eq!(display_after("1.2.3"), "1.23");
    }

    #[test]
    fn format_number_cases() {
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-1.25), "-1.25");
        assert_eq!(format_number(999_999_999_999_999.0), "999999999999999");
        assert_eq!(format_number(1e15), "1e15");
        assert_eq!(format_number(-1e-11), "-1e-11");
        assert_eq!(format_number(1.23456789e-8), "1.23456789e-8");
    }

    // 지수 표기 결과로 이어서 계산할 수 있어야 함
    #[test]
    fn scientific_result_can_be_used_as_operand() {
        assert_eq!(display_after("0.00000001/1000=*1000="), "1e-8");
    }

    // ===== 에러 상태 (4단계) =====

    #[test]
    fn calculate_rejects_divide_by_zero_and_overflow() {
        assert_eq!(calculate(6.0, Operator::Divide, 3.0), Some(2.0));
        assert_eq!(calculate(1.0, Operator::Divide, 0.0), None);
        assert_eq!(calculate(f64::MAX, Operator::Multiply, 2.0), None);
        assert_eq!(calculate(f64::MAX, Operator::Add, f64::MAX), None);
    }

    #[test]
    fn overflow_shows_error() {
        // 약 1e15를 21번 곱하면 약 1e315로 f64 범위(약 1.8e308)를 넘음
        let operands = vec!["999999999999999"; 21].join("*");
        assert_eq!(display_after(&format!("{operands}=")), "Error");
        // 연산자를 누르는 시점에 넘쳐도 Error
        assert_eq!(display_after(&format!("{operands}*")), "Error");
    }

    #[test]
    fn divide_by_zero_mid_chain_shows_error() {
        assert_eq!(display_after("5/0+"), "Error");
    }

    #[test]
    fn inputs_except_digit_and_clear_are_ignored_in_error() {
        assert_eq!(display_after("5/0=+±%="), "Error");
    }

    #[test]
    fn decimal_after_error_starts_fresh() {
        assert_eq!(display_after("5/0=.5"), "0.5");
    }

    #[test]
    fn clear_recovers_from_error() {
        assert_eq!(display_after("5/0=C"), "0");
        assert_eq!(display_after("5/0=C2+3="), "5");
    }
}
