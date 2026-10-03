use iced::{
    alignment,
    keyboard::{self, key::Named, Key, Modifiers},
    widget::{button, column, container, row, text, Button},
    Background, Border, Color, Element, Length, Subscription, Theme,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};

// 계산에 쓰는 숫자: 분수로 정확하게 계산 (예: 1/3 × 3 = 1, 0.1 + 0.2 = 0.3)
type Number = BigRational;

fn main() -> iced::Result {
    iced::application(Calculator::new, Calculator::update, Calculator::view)
        .title("Calculator")
        .theme(Theme::Dark)
        .subscription(Calculator::subscription)
        .window_size((500, 600))
        .run()
}

// 연산자 타입 정의
#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    Add,      // +
    Subtract, // -
    Multiply, // ×
    Divide,   // ÷
}

impl Operator {
    // 화면에 표시할 기호
    fn symbol(self) -> &'static str {
        match self {
            Operator::Add => "+",
            Operator::Subtract => "-",
            Operator::Multiply => "×",
            Operator::Divide => "÷",
        }
    }
}

// 버튼 타입 정의
#[derive(Debug, Clone, PartialEq)]
enum ButtonType {
    Number(char),        // 0-9
    Decimal,             // .
    Operator(Operator),  // +, -, ×, ÷
    Equals,              // =
    Clear,               // C / AC (입력 중이면 현재 입력만, 아니면 전체 초기화)
    AllClear,            // 전체 초기화 (키보드 Esc)
    Sign,                // ±
    Percent,             // %
    Backspace,           // ⌫ (키보드 전용)
}

#[derive(Debug, Clone)]
enum Message {
    ButtonPressed(ButtonType), // 버튼 클릭 또는 대응하는 키 입력
}

// 입력 가능한 최대 자릿수 (디스플레이 폭 한계)
const MAX_INPUT_DIGITS: usize = 15;

// 이 범위를 벗어나는 값은 지수 표기로 표시
const SCIENTIFIC_UPPER: f64 = 1e15;
const SCIENTIFIC_LOWER: f64 = 1e-7;

// 레이아웃 크기
const BUTTON_WIDTH: f32 = 100.0;
const BUTTON_HEIGHT: f32 = 75.0;
const SPACING: f32 = 10.0;
const GRID_WIDTH: f32 = BUTTON_WIDTH * 4.0 + SPACING * 3.0;

struct Calculator {
    display: String,               // 현재 표시할 값
    result_value: Option<Number>,  // 표시 중인 계산 결과의 정확한 값 (새 입력 시 None)
    previous_value: Option<Number>, // 이전 값
    operator: Option<Operator>,    // 현재 연산자
    waiting_for_operand: bool,     // 다음 숫자 입력 시 새로 시작하는지
    operator_just_pressed: bool,   // 직전 입력이 연산자였는지
    error: bool,                   // 에러 상태인지 (0으로 나누기, 오버플로)
    last_operation: Option<(Operator, Number)>, // = 반복용 마지막 연산과 오른쪽 피연산자
}

impl Calculator {
    fn new() -> Self {
        Calculator {
            display: "0".to_string(),
            result_value: None,
            previous_value: None,
            operator: None,
            waiting_for_operand: false,
            operator_just_pressed: false,
            error: false,
            last_operation: None,
        }
    }

    // 상태 초기화
    fn clear(&mut self) {
        self.display = "0".to_string();
        self.result_value = None;
        self.previous_value = None;
        self.operator = None;
        self.waiting_for_operand = false;
        self.operator_just_pressed = false;
        self.error = false;
        self.last_operation = None;
    }

    // 입력 중인 숫자가 있는지 (C/AC 구분 기준)
    fn has_entry(&self) -> bool {
        !self.error && !self.waiting_for_operand && self.display != "0"
    }

    // C: 현재 입력만 지움 (걸려 있는 연산은 유지), AC: 전체 초기화
    fn clear_entry_or_all(&mut self) {
        if self.has_entry() {
            self.display = "0".to_string();
        } else {
            self.clear();
        }
    }

    // C/AC 버튼 라벨
    fn clear_label(&self) -> &'static str {
        if self.has_entry() {
            "C"
        } else {
            "AC"
        }
    }

    // 버튼(또는 대응하는 키) 입력 처리
    fn press(&mut self, button_type: ButtonType) {
        // Backspace는 지울 입력이 없으면 아무것도 하지 않으므로 상태를 유지
        let operator_just_pressed = match button_type {
            ButtonType::Operator(_) => true,
            ButtonType::Backspace => self.operator_just_pressed,
            _ => false,
        };
        match button_type {
            ButtonType::Clear => self.clear_entry_or_all(),
            ButtonType::AllClear => self.clear(),
            ButtonType::Number(digit) => self.input_number(digit),
            ButtonType::Decimal => self.input_decimal(),
            ButtonType::Operator(op) => self.input_operator(op),
            ButtonType::Equals => self.execute_calculation(),
            ButtonType::Sign => self.toggle_sign(),
            ButtonType::Percent => self.calculate_percent(),
            ButtonType::Backspace => self.backspace(),
        }
        self.operator_just_pressed = operator_just_pressed;
    }

    // 화면에 보여줄 문자열
    fn display_text(&self) -> &str {
        if self.error {
            "Error"
        } else {
            &self.display
        }
    }

    // 강조 표시할 연산자 (연산자를 누른 뒤 다음 숫자를 입력하기 전까지)
    fn active_operator(&self) -> Option<Operator> {
        if self.error || !self.waiting_for_operand {
            return None;
        }
        self.operator
    }

    // 디스플레이 위쪽에 보여줄 대기 중인 수식 (예: "5 ×")
    fn expression_text(&self) -> String {
        match (self.error, &self.previous_value, self.operator) {
            (false, Some(prev_val), Some(op)) => {
                format!("{} {}", format_value(prev_val), op.symbol())
            }
            _ => String::new(),
        }
    }

    // 디스플레이 값을 숫자로 변환
    fn get_display_value(&self) -> Number {
        // 계산 결과는 화면용으로 반올림되어 있으므로 정확한 값을 사용
        if let Some(value) = &self.result_value {
            return value.clone();
        }
        parse_number(&self.display)
    }

    // 디스플레이 업데이트 (숫자 포맷팅)
    fn update_display(&mut self, value: Number) {
        self.display = format_value(&value);
        self.result_value = Some(value);
    }

    // 숫자 입력 처리
    fn input_number(&mut self, digit: char) {
        // Error 상태이면 초기화
        if self.error {
            self.clear();
        }
        // 새 입력이 시작되므로 이전 결과 값은 버림
        self.result_value = None;
        
        if self.waiting_for_operand {
            self.display = digit.to_string();
            self.waiting_for_operand = false;
        } else if self.display == "0" {
            self.display = digit.to_string();
        } else if self.display == "-0" {
            self.display = format!("-{digit}");
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
        // 새 입력이 시작되므로 이전 결과 값은 버림
        self.result_value = None;
        
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
            if let Some(prev_val) = &self.previous_value {
                // Error가 발생하면 연산자 설정하지 않음
                let Some(result) = calculate(prev_val, prev_op, &current_value) else {
                    self.error = true;
                    return;
                };
                self.previous_value = Some(result.clone());
                self.update_display(result);
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
        
        let (left, op, right) =
            if let (Some(op), Some(prev_val)) = (self.operator, &self.previous_value) {
                (prev_val.clone(), op, self.get_display_value())
            } else if let Some((op, right)) = &self.last_operation {
                // = 반복: 마지막 연산을 현재 값에 다시 적용
                (self.get_display_value(), *op, right.clone())
            } else {
                return;
            };

        match calculate(&left, op, &right) {
            Some(result) => {
                self.update_display(result);
                self.previous_value = None;
                self.operator = None;
                self.waiting_for_operand = true;
                self.last_operation = Some((op, right));
            }
            None => self.error = true,
        }
    }

    // 한 글자 지우기 (⌫)
    fn backspace(&mut self) {
        // 에러 또는 결과 표시 중이면 지울 입력이 없음
        if self.error || self.waiting_for_operand {
            return;
        }

        self.display.pop();
        if matches!(self.display.as_str(), "" | "-" | "-0") {
            self.display = "0".to_string();
        }
    }

    // 부호 변경 (±)
    fn toggle_sign(&mut self) {
        // Error 상태이면 무시
        if self.error {
            return;
        }

        // 연산자 직후에는 음수 입력을 새로 시작 (예: 5 + ± → -0)
        if self.operator_just_pressed {
            self.display = "-0".to_string();
            self.result_value = None;
            self.waiting_for_operand = false;
            return;
        }
        
        // 문자열에서 부호만 바꿔 입력 중인 형태(예: "1.")를 유지
        if let Some(positive) = self.display.strip_prefix('-') {
            self.display = positive.to_string();
        } else if self.display != "0" {
            self.display.insert(0, '-');
        }
        // 계산 결과를 표시 중이면 정확한 값의 부호도 변경
        if let Some(value) = self.result_value.as_mut() {
            *value = -&*value;
        }
    }

    // 퍼센트 계산 (%)
    fn calculate_percent(&mut self) {
        // Error 상태이면 무시
        if self.error {
            return;
        }
        
        let value = self.get_display_value();
        let hundred = Number::from_integer(BigInt::from(100));
        // +, - 연산 중이면 앞 값 기준 (예: 200 + 10% → 20), 그 외에는 /100
        let percent = match (self.operator, &self.previous_value) {
            (Some(Operator::Add | Operator::Subtract), Some(prev_val)) => prev_val * value / hundred,
            _ => value / hundred,
        };
        if !is_displayable(&percent) {
            self.error = true;
            return;
        }
        self.update_display(percent);
        // 결과 뒤 숫자 입력은 새 입력으로 시작
        self.waiting_for_operand = true;
    }
}

// 계산 실행 (0으로 나누기, 표시 범위 초과 시 None)
fn calculate(left: &Number, op: Operator, right: &Number) -> Option<Number> {
    let result = match op {
        Operator::Add => left + right,
        Operator::Subtract => left - right,
        Operator::Multiply => left * right,
        Operator::Divide => {
            if right.is_zero() {
                return None;
            }
            left / right
        }
    };
    Some(result).filter(is_displayable)
}

// 화면에 표시할 수 있는 크기인지 (f64 범위, 약 1.8e308 이내)
fn is_displayable(value: &Number) -> bool {
    to_display_f64(value).is_some()
}

// 표시용 f64로 변환 (범위를 넘으면 None)
fn to_display_f64(value: &Number) -> Option<f64> {
    value.to_f64().filter(|f| f.is_finite())
}

// 입력 문자열을 정확한 분수로 변환 (예: "-1.25" → -125/100)
fn parse_number(s: &str) -> Number {
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (integer_part, fraction_part) = digits.split_once('.').unwrap_or((digits, ""));
    let numerator: BigInt = format!("{integer_part}{fraction_part}").parse().unwrap_or_default();
    let denominator = BigInt::from(10).pow(fraction_part.len() as u32);
    let value = Number::new(numerator, denominator);
    if negative {
        -value
    } else {
        value
    }
}

// 계산 값을 디스플레이용 문자열로 변환
fn format_value(value: &Number) -> String {
    format_number(to_display_f64(value).expect("계산 결과는 항상 표시 범위 안"))
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

// 키보드 문자를 버튼으로 변환
fn char_to_button(c: char) -> Option<ButtonType> {
    let button_type = match c {
        '0'..='9' => ButtonType::Number(c),
        '.' | ',' => ButtonType::Decimal,
        '+' => ButtonType::Operator(Operator::Add),
        '-' => ButtonType::Operator(Operator::Subtract),
        '*' | 'x' | 'X' | '×' => ButtonType::Operator(Operator::Multiply),
        '/' | '÷' => ButtonType::Operator(Operator::Divide),
        '=' => ButtonType::Equals,
        '%' => ButtonType::Percent,
        'c' | 'C' => ButtonType::Clear,
        _ => return None,
    };
    Some(button_type)
}

// 특수 키를 버튼으로 변환
fn named_key_to_button(named: Named) -> Option<ButtonType> {
    match named {
        Named::Enter => Some(ButtonType::Equals),
        Named::Escape => Some(ButtonType::AllClear),
        Named::Backspace => Some(ButtonType::Backspace),
        _ => None,
    }
}

// 키 입력을 버튼으로 변환
fn key_to_button(key: &Key, text: Option<&str>, modifiers: Modifiers) -> Option<ButtonType> {
    if is_shortcut(modifiers) {
        return None;
    }
    // Enter, Esc, Backspace는 키 이름 기준 (입력 문자가 "\r" 등이어도 중복 처리 안 됨)
    if let Key::Named(named) = key {
        if let Some(button_type) = named_key_to_button(*named) {
            return Some(button_type);
        }
    }
    // 그 외에는 실제 입력된 문자 기준 (키보드 배열 무관, 예: Shift+8 = '*')
    let mut chars = text?.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    char_to_button(c)
}

// Cmd/Ctrl/Alt가 눌렸으면 단축키이므로 계산기 입력으로 처리하지 않음
fn is_shortcut(modifiers: Modifiers) -> bool {
    modifiers.logo() || modifiers.control() || modifiers.alt()
}

// 문자열에 포함된 숫자 자릿수 (부호, 소수점 제외)
fn count_digits(s: &str) -> usize {
    s.chars().filter(|c| c.is_ascii_digit()).count()
}

impl Calculator {
    fn update(&mut self, message: Message) {
        match message {
            Message::ButtonPressed(button_type) => self.press(button_type),
        }
    }

    // 키보드 이벤트 구독
    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, status, _window| {
            // 위젯이 이미 처리한 이벤트는 무시
            if status == iced::event::Status::Captured {
                return None;
            }
            let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key, text, modifiers, ..
            }) = event
            else {
                return None;
            };
            key_to_button(&key, text.as_deref(), modifiers).map(Message::ButtonPressed)
        })
    }

    fn view(&self) -> Element<'_, Message> {
        // 메인 컬럼: 디스플레이 + 버튼 그리드
        let content = column![
            // 디스플레이 영역
            display_area(self.display_text(), self.expression_text()),
            // 버튼 그리드
            button_grid(self.active_operator(), self.clear_label())
        ]
        .spacing(SPACING);

        // 창 크기가 바뀌어도 계산기를 가운데에 배치
        container(content).center(Length::Fill).into()
    }
}

// 디스플레이 영역 컴포넌트 (개선: 긴 숫자 처리)
fn display_area(value: &str, expression: String) -> Element<'_, Message> {
    // 긴 숫자에 대해 폰트 크기 자동 조정
    let font_size = if value.len() > 12 {
        32.0
    } else if value.len() > 8 {
        40.0
    } else {
        48.0
    };

    // 위: 대기 중인 수식 (작고 흐리게), 아래: 현재 값
    let expression = text(expression)
        .size(20)
        .color(Color::from_rgb(0.6, 0.6, 0.6))
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Right);

    let value = text(value)
        .size(font_size)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Center);

    container(column![expression, value])
        .width(Length::Fixed(GRID_WIDTH))
        .height(Length::Fixed(120.0))
        .padding([10, 20])
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
            border: Border {
                color: Color::from_rgb(0.4, 0.4, 0.4),
                width: 2.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// 버튼 스타일 타입
#[derive(Clone, Copy)]
enum ButtonStyleType {
    Number,    // 숫자 버튼
    Operator,  // 연산자 버튼
    ActiveOperator, // 선택된 연산자 버튼
    Function,  // 기능 버튼 (C, ±, %)
    Equals,    // = 버튼
}

// 버튼 스타일
fn button_style(style_type: ButtonStyleType, status: button::Status) -> button::Style {
    let (background, text_color) = match style_type {
        ButtonStyleType::Number => {
            // 숫자 버튼: 어두운 회색
            (Color::from_rgb(0.3, 0.3, 0.3), Color::WHITE)
        }
        ButtonStyleType::Operator => {
            // 연산자 버튼: 주황색
            (Color::from_rgb(1.0, 0.6, 0.0), Color::WHITE)
        }
        ButtonStyleType::ActiveOperator => {
            // 선택된 연산자 버튼: 색 반전 (흰 배경, 주황 글자)
            (Color::WHITE, Color::from_rgb(1.0, 0.6, 0.0))
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

    // 호버 시 약간 밝게, 눌렀을 때 약간 어둡게
    let background = match status {
        button::Status::Hovered => adjust_brightness(background, 0.1),
        button::Status::Pressed => adjust_brightness(background, -0.1),
        button::Status::Active | button::Status::Disabled => background,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border::default().rounded(10),
        ..Default::default()
    }
}

// 색의 밝기를 조정 (0.0 ~ 1.0 범위 유지)
fn adjust_brightness(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).clamp(0.0, 1.0),
        g: (color.g + amount).clamp(0.0, 1.0),
        b: (color.b + amount).clamp(0.0, 1.0),
        a: color.a,
    }
}

// 버튼 그리드 레이아웃
fn button_grid(
    active_operator: Option<Operator>,
    clear_label: &'static str,
) -> Element<'static, Message> {
    // 연산자 버튼 (선택된 연산자는 강조)
    let operator_button = |op: Operator| {
        let style_type = if active_operator == Some(op) {
            ButtonStyleType::ActiveOperator
        } else {
            ButtonStyleType::Operator
        };
        calc_button(op.symbol(), ButtonType::Operator(op), style_type)
    };

    column![
        // 첫 번째 행: C(AC), ±, %, ÷
        row![
            calc_button(clear_label, ButtonType::Clear, ButtonStyleType::Function),
            calc_button("±", ButtonType::Sign, ButtonStyleType::Function),
            calc_button("%", ButtonType::Percent, ButtonStyleType::Function),
            operator_button(Operator::Divide),
        ]
        .spacing(SPACING),
        // 두 번째 행: 7, 8, 9, ×
        row![
            calc_button("7", ButtonType::Number('7'), ButtonStyleType::Number),
            calc_button("8", ButtonType::Number('8'), ButtonStyleType::Number),
            calc_button("9", ButtonType::Number('9'), ButtonStyleType::Number),
            operator_button(Operator::Multiply),
        ]
        .spacing(SPACING),
        // 세 번째 행: 4, 5, 6, -
        row![
            calc_button("4", ButtonType::Number('4'), ButtonStyleType::Number),
            calc_button("5", ButtonType::Number('5'), ButtonStyleType::Number),
            calc_button("6", ButtonType::Number('6'), ButtonStyleType::Number),
            operator_button(Operator::Subtract),
        ]
        .spacing(SPACING),
        // 네 번째 행: 1, 2, 3, +
        row![
            calc_button("1", ButtonType::Number('1'), ButtonStyleType::Number),
            calc_button("2", ButtonType::Number('2'), ButtonStyleType::Number),
            calc_button("3", ButtonType::Number('3'), ButtonStyleType::Number),
            operator_button(Operator::Add),
        ]
        .spacing(SPACING),
        // 다섯 번째 행: 0 (버튼 2칸 + 간격), ., =
        row![
            calc_button("0", ButtonType::Number('0'), ButtonStyleType::Number)
                .width(Length::Fixed(BUTTON_WIDTH * 2.0 + SPACING)),
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
) -> Button<'a, Message> {
    // 라벨을 버튼 가운데에 배치
    let label = text(label)
        .size(28)
        .width(Length::Fill)
        .height(Length::Fill)
        .center();

    button(label)
        .width(Length::Fixed(BUTTON_WIDTH))
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(move |_theme, status| button_style(style_type, status))
        .on_press(Message::ButtonPressed(button_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_calc() -> Calculator {
        Calculator::new()
    }

    // 키 문자열을 버튼 입력으로 변환해 순서대로 누름
    // 0-9 . + - * / = C ± % ⌫
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
                '⌫' => ButtonType::Backspace,
                _ => panic!("알 수 없는 키: {key}"),
            };
            calc.update(Message::ButtonPressed(button_type));
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
        // C(입력 지움) 후 AC(전체 초기화)
        assert_eq!(display_after("12+3CC"), "0");
        assert_eq!(display_after("12+3CC4="), "4");
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
        assert_eq!(display_after("200+10%+"), "220");
        assert_eq!(display_after("200+10%+5="), "225");
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

    fn num(s: &str) -> Number {
        parse_number(s)
    }

    #[test]
    fn calculate_rejects_divide_by_zero_and_overflow() {
        assert_eq!(calculate(&num("6"), Operator::Divide, &num("3")), Some(num("2")));
        assert_eq!(calculate(&num("1"), Operator::Divide, &num("0")), None);
        // 1e300 × 1e10 = 1e310은 표시 범위(약 1.8e308)를 넘음
        let huge = Number::from_integer(BigInt::from(10).pow(300));
        assert_eq!(calculate(&huge, Operator::Multiply, &num("10000000000")), None);
        assert!(calculate(&huge, Operator::Multiply, &num("100")).is_some());
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

    // ===== = 반복 (5단계) =====

    #[test]
    fn equals_repeats_last_operation() {
        assert_eq!(display_after("5+3==="), "14");
        assert_eq!(display_after("10-2=="), "6");
        assert_eq!(display_after("2*3=="), "18");
        assert_eq!(display_after("81/3=="), "9");
    }

    #[test]
    fn equals_right_after_operator_uses_same_value() {
        assert_eq!(display_after("5+="), "10");
        assert_eq!(display_after("5+=="), "15");
    }

    #[test]
    fn equals_repeat_applies_to_new_input() {
        assert_eq!(display_after("5+3=10="), "13");
    }

    #[test]
    fn equals_without_operation_does_nothing() {
        assert_eq!(display_after("7="), "7");
        assert_eq!(display_after("7=="), "7");
    }

    #[test]
    fn clear_forgets_last_operation() {
        assert_eq!(display_after("5+3=C="), "0");
        assert_eq!(display_after("5+3=C7="), "7");
    }

    #[test]
    fn new_operation_replaces_last_operation() {
        assert_eq!(display_after("5+3=*2=="), "32");
    }

    #[test]
    fn equals_repeat_can_hit_error() {
        assert_eq!(display_after("5/0=="), "Error");
        let operands = vec!["999999999999999"; 20].join("*");
        // 약 1e300에 1e15를 한 번 더 곱하면 오버플로
        assert_eq!(display_after(&format!("{operands}=")), "1e300");
        assert_eq!(display_after(&format!("{operands}=*999999999999999=")), "Error");
    }

    // ===== Backspace (5단계) =====

    #[test]
    fn backspace_removes_last_character() {
        assert_eq!(display_after("123⌫"), "12");
        assert_eq!(display_after("1.5⌫"), "1.");
        assert_eq!(display_after("1.⌫"), "1");
    }

    #[test]
    fn backspace_to_empty_shows_zero() {
        assert_eq!(display_after("5⌫"), "0");
        assert_eq!(display_after("⌫"), "0");
        assert_eq!(display_after("12±⌫⌫"), "0");
        assert_eq!(display_after("0.±⌫"), "0");
    }

    #[test]
    fn backspace_then_continue_typing() {
        assert_eq!(display_after("129⌫3+1="), "124");
    }

    #[test]
    fn backspace_does_not_edit_result() {
        assert_eq!(display_after("2+3=⌫"), "5");
        assert_eq!(display_after("50%⌫"), "0.5");
    }

    #[test]
    fn backspace_after_operator_keeps_operator_replacement() {
        assert_eq!(display_after("5+⌫"), "5");
        assert_eq!(display_after("5+⌫*3="), "15");
    }

    #[test]
    fn backspace_ignored_in_error() {
        assert_eq!(display_after("5/0=⌫"), "Error");
    }

    // ===== 키보드 입력 (5단계) =====

    #[test]
    fn char_mapping() {
        assert_eq!(char_to_button('7'), Some(ButtonType::Number('7')));
        assert_eq!(char_to_button('.'), Some(ButtonType::Decimal));
        assert_eq!(char_to_button(','), Some(ButtonType::Decimal));
        assert_eq!(char_to_button('*'), Some(ButtonType::Operator(Operator::Multiply)));
        assert_eq!(char_to_button('x'), Some(ButtonType::Operator(Operator::Multiply)));
        assert_eq!(char_to_button('/'), Some(ButtonType::Operator(Operator::Divide)));
        assert_eq!(char_to_button('='), Some(ButtonType::Equals));
        assert_eq!(char_to_button('c'), Some(ButtonType::Clear));
        // Enter, Backspace가 문자로도 들어오는 경우 중복 처리되지 않아야 함
        assert_eq!(char_to_button('\r'), None);
        assert_eq!(char_to_button('\u{7f}'), None);
        assert_eq!(char_to_button('\u{8}'), None);
        assert_eq!(char_to_button('a'), None);
    }

    fn key(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn named_key_mapping() {
        let none = Modifiers::default();
        // 특수 키는 입력 문자("\r" 등)가 함께 와도 키 이름 기준으로 한 번만 처리
        assert_eq!(key_to_button(&Key::Named(Named::Enter), Some("\r"), none), Some(ButtonType::Equals));
        assert_eq!(key_to_button(&Key::Named(Named::Escape), Some("\u{1b}"), none), Some(ButtonType::AllClear));
        assert_eq!(key_to_button(&Key::Named(Named::Backspace), Some("\u{8}"), none), Some(ButtonType::Backspace));
        assert_eq!(key_to_button(&Key::Named(Named::Tab), Some("\t"), none), None);
    }

    #[test]
    fn typed_text_mapping() {
        let none = Modifiers::default();
        assert_eq!(key_to_button(&key("7"), Some("7"), none), Some(ButtonType::Number('7')));
        // Shift+8은 키는 '8'이지만 입력 문자는 '*'
        assert_eq!(
            key_to_button(&key("8"), Some("*"), Modifiers::SHIFT),
            Some(ButtonType::Operator(Operator::Multiply))
        );
        assert_eq!(key_to_button(&key("a"), Some("a"), none), None);
        assert_eq!(key_to_button(&key("a"), None, none), None);
        assert_eq!(key_to_button(&key("ab"), Some("ab"), none), None);
    }

    #[test]
    fn shortcuts_are_ignored() {
        // Cmd+C(복사)가 C(초기화)로 처리되면 안 됨
        assert_eq!(key_to_button(&key("c"), Some("c"), Modifiers::COMMAND), None);
        assert_eq!(key_to_button(&key("5"), Some("5"), Modifiers::CTRL), None);
        assert_eq!(key_to_button(&key("5"), Some("5"), Modifiers::ALT), None);
        assert_eq!(key_to_button(&Key::Named(Named::Backspace), None, Modifiers::COMMAND), None);
    }

    // 키 입력을 구독과 같은 경로로 처리
    fn send_key(calc: &mut Calculator, key: Key, text: Option<&str>) {
        if let Some(button_type) = key_to_button(&key, text, Modifiers::default()) {
            calc.update(Message::ButtonPressed(button_type));
        }
    }

    fn type_chars(calc: &mut Calculator, chars: &str) {
        for c in chars.chars() {
            let c = c.to_string();
            send_key(calc, key(&c), Some(&c));
        }
    }

    fn press_key(calc: &mut Calculator, named: Named) {
        send_key(calc, Key::Named(named), None);
    }

    #[test]
    fn keyboard_calculation() {
        let mut calc = new_calc();
        type_chars(&mut calc, "12*3");
        press_key(&mut calc, Named::Enter);
        assert_eq!(calc.display_text(), "36");
    }

    #[test]
    fn keyboard_backspace_and_escape() {
        let mut calc = new_calc();
        type_chars(&mut calc, "123");
        press_key(&mut calc, Named::Backspace);
        assert_eq!(calc.display_text(), "12");
        press_key(&mut calc, Named::Escape);
        assert_eq!(calc.display_text(), "0");
    }

    // ===== 선택된 연산자 표시 =====

    fn state_after(keys: &str) -> Calculator {
        let mut calc = new_calc();
        press(&mut calc, keys);
        calc
    }

    #[test]
    fn operator_is_highlighted_until_next_input() {
        assert_eq!(state_after("5+").active_operator(), Some(Operator::Add));
        assert_eq!(state_after("5+3").active_operator(), None);
        assert_eq!(state_after("5+.").active_operator(), None);
        assert_eq!(state_after("5").active_operator(), None);
    }

    #[test]
    fn highlight_follows_replaced_operator() {
        assert_eq!(state_after("5+*").active_operator(), Some(Operator::Multiply));
        assert_eq!(state_after("5+3*").active_operator(), Some(Operator::Multiply));
        // Backspace는 지울 입력이 없으므로 강조 유지
        assert_eq!(state_after("5+⌫").active_operator(), Some(Operator::Add));
    }

    #[test]
    fn no_highlight_after_equals_clear_or_error() {
        assert_eq!(state_after("5+3=").active_operator(), None);
        assert_eq!(state_after("5+C").active_operator(), None);
        assert_eq!(state_after("5/0+").active_operator(), None);
    }

    #[test]
    fn expression_shows_pending_operation() {
        assert_eq!(state_after("5+").expression_text(), "5 +");
        // 다음 숫자를 입력하는 중에도 유지
        assert_eq!(state_after("5+3").expression_text(), "5 +");
        // 연속 계산 시 중간 결과로 갱신
        assert_eq!(state_after("5+3*").expression_text(), "8 ×");
        assert_eq!(state_after("8/").expression_text(), "8 ÷");
        assert_eq!(state_after("8-").expression_text(), "8 -");
        assert_eq!(state_after("50%+").expression_text(), "0.5 +");
    }

    #[test]
    fn expression_follows_replaced_operator() {
        assert_eq!(state_after("5+*").expression_text(), "5 ×");
    }

    #[test]
    fn expression_is_empty_without_pending_operation() {
        assert_eq!(state_after("").expression_text(), "");
        assert_eq!(state_after("5").expression_text(), "");
        assert_eq!(state_after("5+3=").expression_text(), "");
        assert_eq!(state_after("5+C").expression_text(), "");
        assert_eq!(state_after("5/0=").expression_text(), "");
    }

    // ===== 버그 8: = 결과를 이어서 계산할 때 정밀도 유지 =====

    #[test]
    fn bug8_result_keeps_precision_for_next_operation() {
        assert_eq!(display_after("1/3=*3="), "1");
        assert_eq!(display_after("2/3=*3="), "2");
    }

    #[test]
    fn bug8_equals_repeat_keeps_precision() {
        assert_eq!(display_after("10/3==*9="), "10");
    }

    #[test]
    fn bug8_percent_result_keeps_precision() {
        assert_eq!(display_after("10/3=%*300="), "10");
    }

    #[test]
    fn bug8_toggle_sign_keeps_precision() {
        assert_eq!(display_after("1/3=±*3="), "-1");
        assert_eq!(display_after("1/3=±±*3="), "1");
    }

    #[test]
    fn bug8_new_input_replaces_result() {
        assert_eq!(display_after("1/3=5*3="), "15");
        assert_eq!(display_after("1/3=.5*2="), "1");
    }

    // 원래 값을 쓰더라도 부동소수점 오차는 결과에 남지 않아야 함
    #[test]
    fn float_noise_does_not_leak_into_results() {
        assert_eq!(display_after("0.1+0.2=-0.3="), "0");
        assert_eq!(display_after("0.1+0.2-0.3="), "0");
        assert_eq!(display_after("0.1*3=-0.3="), "0");
        assert_eq!(display_after("1.1*1.1=-1.21="), "0");
    }

    #[test]
    fn parse_number_is_exact() {
        assert_eq!(num("0"), Number::zero());
        assert_eq!(num("-0"), Number::zero());
        assert_eq!(num("1.25"), Number::new(BigInt::from(5), BigInt::from(4)));
        assert_eq!(num("-1.25"), Number::new(BigInt::from(-5), BigInt::from(4)));
        assert_eq!(num("0.1") + num("0.2"), num("0.3"));
        // 입력 중인 형태도 처리
        assert_eq!(num("1."), num("1"));
        assert_eq!(num("-0."), Number::zero());
    }

    // ===== 유리수 계산: 나눗셈 결과도 정확 =====

    #[test]
    fn division_results_are_exact() {
        assert_eq!(display_after("1/3=*3=-1="), "0");
        assert_eq!(display_after("1/3*3-1="), "0");
        assert_eq!(display_after("1/7=*7="), "1");
    }

    #[test]
    fn repeated_division_and_multiplication_round_trip() {
        // 3으로 10번 나눈 뒤 3으로 10번 곱하면 원래 값
        let keys = format!("10/3{}*3{}", "=".repeat(10), "=".repeat(10));
        assert_eq!(display_after(&keys), "10");
    }

    // ===== 문맥 퍼센트 =====

    #[test]
    fn percent_with_add_or_subtract_uses_previous_value() {
        assert_eq!(display_after("200+10%"), "20");
        assert_eq!(display_after("200+10%="), "220");
        assert_eq!(display_after("200-10%="), "180");
        // 피연산자 없이 %를 누르면 앞 값 자신을 기준으로 계산
        assert_eq!(display_after("200+%"), "400");
    }

    #[test]
    fn percent_with_multiply_or_divide_divides_by_hundred() {
        assert_eq!(display_after("200*10%"), "0.1");
        assert_eq!(display_after("200*10%="), "20");
        assert_eq!(display_after("200/10%="), "2000");
    }

    #[test]
    fn percent_without_operator_divides_by_hundred() {
        assert_eq!(display_after("5+3=%"), "0.08");
    }

    // ===== C / AC =====

    #[test]
    fn clear_label_depends_on_entry() {
        assert_eq!(state_after("").clear_label(), "AC");
        assert_eq!(state_after("5").clear_label(), "C");
        assert_eq!(state_after("5+").clear_label(), "AC");
        assert_eq!(state_after("5+3").clear_label(), "C");
        assert_eq!(state_after("5+3C").clear_label(), "AC");
        assert_eq!(state_after("5+3=").clear_label(), "AC");
        assert_eq!(state_after("5/0=").clear_label(), "AC");
    }

    #[test]
    fn clear_entry_keeps_pending_operation() {
        assert_eq!(display_after("12+3C"), "0");
        assert_eq!(display_after("12+3C4="), "16");
        assert_eq!(state_after("12+3C").expression_text(), "12 +");
    }

    #[test]
    fn all_clear_when_no_entry() {
        assert_eq!(state_after("12+C").expression_text(), "");
        assert_eq!(display_after("12+C4="), "4");
        assert_eq!(display_after("5+3=C="), "0");
    }

    #[test]
    fn escape_always_clears_everything() {
        let mut calc = new_calc();
        type_chars(&mut calc, "12+3");
        press_key(&mut calc, Named::Escape);
        assert_eq!(calc.display_text(), "0");
        assert_eq!(calc.expression_text(), "");
    }

    // ===== 연산자 직후 ± =====

    #[test]
    fn sign_after_operator_starts_negative_input() {
        assert_eq!(display_after("5+±"), "-0");
        assert_eq!(display_after("5+±3"), "-3");
        assert_eq!(display_after("5+±3="), "2");
        assert_eq!(display_after("5*±2="), "-10");
        assert_eq!(display_after("5+±.5="), "4.5");
        // 이전 값과 수식은 그대로
        assert_eq!(state_after("5+±").expression_text(), "5 +");
    }

    #[test]
    fn sign_twice_after_operator_cancels() {
        assert_eq!(display_after("5+±±"), "0");
        assert_eq!(display_after("5+±±3="), "8");
    }

    #[test]
    fn sign_on_percent_result_negates_it() {
        assert_eq!(display_after("200+10%±="), "180");
    }

    #[test]
    fn backspace_on_negative_zero() {
        assert_eq!(display_after("5+±⌫"), "0");
    }
}
