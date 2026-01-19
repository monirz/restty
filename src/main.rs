use eframe::egui;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use chrono::{DateTime, Utc};

const SUPABASE_URL: &str = "https://drtejwkmjuwyqugpdspe.supabase.co";
const SUPABASE_ANON_KEY: &str = "sb_publishable_0zSJqibEWNDVan_BOpvJDg_yYMdp9lO";
const MAX_RESPONSE_SIZE: usize = 100_000; // 100 KB
const MAX_BODY_SIZE: usize = 10_000; // 10 KB

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_title("restty"),
        ..Default::default()
    };

    eframe::run_native("restty", options, Box::new(|cc| {
        setup_custom_style(&cc.egui_ctx);
        Box::new(App::new())
    }))
}

fn setup_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
    style.visuals.panel_fill = egui::Color32::from_rgb(18, 18, 18);
    style.visuals.window_fill = egui::Color32::from_rgb(18, 18, 18);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(10, 10, 10);

    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 30);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 40, 40);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 50);

    style.visuals.selection.bg_fill = egui::Color32::from_rgb(0, 180, 100);
    style.visuals.selection.stroke.color = egui::Color32::from_rgb(0, 200, 120);

    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    user_id: String,
    method: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    status: String,
    response: String,
    time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct SupabaseAuthRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct SupabaseAuthResponse {
    access_token: String,
    user: SupabaseUser,
}

#[derive(Deserialize)]
struct SupabaseUser {
    id: String,
    email: String,
}

#[derive(PartialEq)]
enum View {
    Login,
    Main,
}

struct App {
    view: View,
    token: Option<String>,
    user_id: Option<String>,
    email: Option<String>,

    login_email: String,
    login_password: String,
    login_error: String,
    is_signup: bool,

    url: String,
    method: Method,
    body: String,
    response: String,
    status: String,
    time: String,

    history: Vec<HistoryItem>,
    show_history: bool,
    selected_history_id: Option<String>,

    url_field_focused: bool,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum Method {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl Method {
    fn as_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
        }
    }
}

impl App {
    fn new() -> Self {
        let (token, user_id, email) = load_credentials();
        let view = View::Main;

        let mut app = Self {
            view,
            token: token.clone(),
            user_id: user_id.clone(),
            email: email.clone(),
            login_email: String::new(),
            login_password: String::new(),
            login_error: String::new(),
            is_signup: false,
            url: String::new(),
            method: Method::GET,
            body: String::new(),
            response: String::new(),
            status: String::new(),
            time: String::new(),
            history: Vec::new(),
            show_history: false,
            selected_history_id: None,
            url_field_focused: false,
        };

        if token.is_some() {
            app.show_history = true;
            app.load_history();
        }

        app
    }

    fn login(&mut self) {
        if self.is_signup {
            self.signup_then_login();
        } else {
            self.do_login();
        }
    }

    fn signup_then_login(&mut self) {
        let client = Client::new();
        let signup_url = format!("{}/auth/v1/signup", SUPABASE_URL);

        let req = SupabaseAuthRequest {
            email: self.login_email.clone(),
            password: self.login_password.clone(),
        };

        eprintln!("Attempting signup...");
        match client.post(&signup_url)
            .header("apikey", SUPABASE_ANON_KEY)
            .header("Content-Type", "application/json")
            .json(&req)
            .send() {
            Ok(resp) => {
                let status = resp.status();
                eprintln!("Signup response status: {}", status);

                if status.is_success() {
                    let body = resp.text().unwrap_or_default();
                    eprintln!("Signup response body: {}", body);

                    // Check if email confirmation is required
                    if body.contains("confirmation_sent_at") {
                        self.login_error = "Please check your email to confirm your account before logging in.".to_string();
                        return;
                    }

                    // Signup successful, now login
                    eprintln!("Signup successful, attempting login...");
                    self.is_signup = false;
                    self.do_login();
                } else {
                    let error_body = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
                    eprintln!("Signup error: {}", error_body);

                    // Check if user already exists
                    if error_body.contains("already registered") || error_body.contains("already been registered") {
                        self.login_error = "User already exists. Please use Login instead.".to_string();
                    } else {
                        self.login_error = format!("Signup failed: {}", error_body);
                    }
                }
            }
            Err(e) => {
                eprintln!("Signup connection error: {}", e);
                self.login_error = format!("Connection error: {}", e);
            }
        }
    }

    fn do_login(&mut self) {
        let client = Client::new();
        let login_url = format!("{}/auth/v1/token?grant_type=password", SUPABASE_URL);

        let req = SupabaseAuthRequest {
            email: self.login_email.clone(),
            password: self.login_password.clone(),
        };

        eprintln!("Attempting login...");
        match client.post(&login_url)
            .header("apikey", SUPABASE_ANON_KEY)
            .header("Content-Type", "application/json")
            .json(&req)
            .send() {
            Ok(resp) => {
                let status = resp.status();
                eprintln!("Login response status: {}", status);

                if status.is_success() {
                    match resp.json::<SupabaseAuthResponse>() {
                        Ok(auth_resp) => {
                            eprintln!("Login successful!");
                            self.token = Some(auth_resp.access_token.clone());
                            self.user_id = Some(auth_resp.user.id.clone());
                            self.email = Some(auth_resp.user.email.clone());
                            save_credentials(&auth_resp.access_token, &auth_resp.user.id, &auth_resp.user.email);
                            self.view = View::Main;
                            self.show_history = true;
                            self.login_error.clear();
                            self.load_history();
                        }
                        Err(e) => {
                            eprintln!("Login parse error: {}", e);
                            self.login_error = format!("Failed to parse login response: {}", e);
                        }
                    }
                } else {
                    let error_body = resp.text().unwrap_or_else(|_| "Unknown error".to_string());
                    eprintln!("Login error: {}", error_body);

                    if error_body.contains("Email not confirmed") || error_body.contains("email_not_confirmed") {
                        self.login_error = "Please confirm your email before logging in. Check your inbox.".to_string();
                    } else {
                        self.login_error = "Invalid email or password".to_string();
                    }
                }
            }
            Err(e) => {
                eprintln!("Login connection error: {}", e);
                self.login_error = format!("Connection error: {}", e);
            }
        }
    }

    fn logout(&mut self) {
        self.token = None;
        self.user_id = None;
        self.email = None;
        self.view = View::Main;
        self.show_history = false;
        self.history.clear();
        clear_credentials();
    }

    fn load_history(&mut self) {
        if let (Some(token), Some(user_id)) = (&self.token, &self.user_id) {
            let client = Client::new();
            let url = format!("{}/rest/v1/history?user_id=eq.{}&order=created_at.desc&limit=100", SUPABASE_URL, user_id);

            match client.get(&url)
                .header("apikey", SUPABASE_ANON_KEY)
                .header("Authorization", format!("Bearer {}", token))
                .send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(history) = resp.json::<Vec<HistoryItem>>() {
                            self.history = history;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load history: {}", e);
                }
            }
        }
    }

    fn save_to_history(&mut self) {
        if let (Some(token), Some(user_id)) = (&self.token, &self.user_id) {
            // Truncate body and response to prevent flooding database
            let truncated_body = if self.body.is_empty() {
                None
            } else {
                Some(truncate_string(&self.body, MAX_BODY_SIZE))
            };

            let truncated_response = truncate_string(&self.response, MAX_RESPONSE_SIZE);

            let item = HistoryItem {
                id: None,
                user_id: user_id.clone(),
                method: self.method.as_str().to_string(),
                url: self.url.clone(),
                body: truncated_body,
                status: self.status.clone(),
                response: truncated_response,
                time: self.time.clone(),
                created_at: None,
            };

            let client = Client::new();
            match client.post(&format!("{}/rest/v1/history", SUPABASE_URL))
                .header("apikey", SUPABASE_ANON_KEY)
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
                .header("Prefer", "return=representation")
                .json(&item)
                .send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(mut saved_items) = resp.json::<Vec<HistoryItem>>() {
                            if let Some(saved_item) = saved_items.pop() {
                                self.history.insert(0, saved_item);
                                if self.history.len() > 100 {
                                    self.history.truncate(100);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to save history: {}", e);
                }
            }
        }
    }

    fn delete_history_item(&mut self, id: &str) {
        if let Some(token) = &self.token {
            let client = Client::new();
            let url = format!("{}/rest/v1/history?id=eq.{}", SUPABASE_URL, id);

            match client.delete(&url)
                .header("apikey", SUPABASE_ANON_KEY)
                .header("Authorization", format!("Bearer {}", token))
                .send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        self.history.retain(|item| {
                            item.id.as_ref().map(|item_id| item_id.as_str()) != Some(id)
                        });
                    }
                }
                Err(e) => {
                    eprintln!("Failed to delete history: {}", e);
                }
            }
        }
    }

    fn load_history_item(&mut self, item: &HistoryItem) {
        self.url = item.url.clone();
        self.method = match item.method.as_str() {
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => Method::GET,
        };
        self.body = item.body.clone().unwrap_or_default();
        self.response = item.response.clone();
        self.status = item.status.clone();
        self.time = item.time.clone();
        self.selected_history_id = item.id.clone();
    }

    fn send_request(&mut self) {
        let client = Client::new();
        let start = Instant::now();

        let result = match self.method {
            Method::GET => client.get(&self.url).send(),
            Method::POST => {
                let mut req = client.post(&self.url);
                if !self.body.is_empty() {
                    req = req.header("Content-Type", "application/json").body(self.body.clone());
                }
                req.send()
            }
            Method::PUT => {
                let mut req = client.put(&self.url);
                if !self.body.is_empty() {
                    req = req.header("Content-Type", "application/json").body(self.body.clone());
                }
                req.send()
            }
            Method::DELETE => client.delete(&self.url).send(),
            Method::PATCH => {
                let mut req = client.patch(&self.url);
                if !self.body.is_empty() {
                    req = req.header("Content-Type", "application/json").body(self.body.clone());
                }
                req.send()
            }
        };

        let duration = start.elapsed();
        self.time = format!("{:.0?}", duration);

        match result {
            Ok(resp) => {
                self.status = resp.status().to_string();
                if let Ok(text) = resp.text() {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        self.response = serde_json::to_string_pretty(&json).unwrap_or(text);
                    } else {
                        self.response = text;
                    }
                }
                if self.token.is_some() {
                    self.save_to_history();
                }
            }
            Err(e) => {
                self.status = "Error".to_string();
                self.response = e.to_string();
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let green = egui::Color32::from_rgb(0, 200, 120);

        ctx.input(|i| {
            if i.key_pressed(egui::Key::L) && i.modifiers.command {
                self.url_field_focused = true;
            }
            if i.key_pressed(egui::Key::H) && i.modifiers.command && self.view == View::Main {
                self.show_history = !self.show_history;
            }
            if i.key_pressed(egui::Key::Enter) && i.modifiers.command && self.view == View::Main {
                if !self.url.is_empty() {
                    self.send_request();
                }
            }
        });

        match self.view {
            View::Login => self.show_login(ctx, green),
            View::Main => self.show_main(ctx, green),
        }
    }
}

impl App {
    fn show_login(&mut self, ctx: &egui::Context, green: egui::Color32) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);

                ui.label(egui::RichText::new("restty").size(48.0).color(green));
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Dev-friendly HTTP client").size(16.0).color(egui::Color32::GRAY));

                ui.add_space(50.0);

                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.is_signup, "Login").clicked() {
                        self.is_signup = false;
                        self.login_error.clear();
                    }
                    if ui.selectable_label(self.is_signup, "Sign Up").clicked() {
                        self.is_signup = true;
                        self.login_error.clear();
                    }
                });

                ui.add_space(20.0);

                ui.label("Email");
                let email_field = egui::TextEdit::singleline(&mut self.login_email)
                    .hint_text("dev@test.com")
                    .desired_width(300.0);
                ui.add(email_field);

                ui.add_space(10.0);

                ui.label("Password");
                let password_field = egui::TextEdit::singleline(&mut self.login_password)
                    .password(true)
                    .hint_text("password123")
                    .desired_width(300.0);
                if ui.add(password_field).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.login();
                }

                ui.add_space(20.0);

                let btn_text = if self.is_signup { "Sign Up" } else { "Login" };
                let login_btn = egui::Button::new(
                    egui::RichText::new(btn_text).color(egui::Color32::BLACK)
                ).fill(green).min_size(egui::vec2(300.0, 35.0));

                if ui.add(login_btn).clicked() {
                    self.login();
                }

                if !self.login_error.is_empty() {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&self.login_error).color(egui::Color32::from_rgb(255, 80, 80)));
                }

                ui.add_space(20.0);

                let skip_btn = egui::Button::new(
                    egui::RichText::new("Continue without login").color(egui::Color32::GRAY)
                ).fill(egui::Color32::from_rgb(40, 40, 40)).min_size(egui::vec2(300.0, 30.0));

                if ui.add(skip_btn).clicked() {
                    self.view = View::Main;
                }

                ui.add_space(20.0);
                ui.label(egui::RichText::new("Login to save and sync request history").size(12.0).color(egui::Color32::DARK_GRAY));
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Hint: Use dev@test.com / password123").size(12.0).color(egui::Color32::DARK_GRAY));
            });
        });
    }

    fn show_main(&mut self, ctx: &egui::Context, green: egui::Color32) {
        if self.show_history {
            egui::SidePanel::left("history_panel")
                .resizable(false)
                .exact_width(350.0)
                .show(ctx, |ui| {
                    self.show_history_panel(ui, green);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("restty").size(24.0).color(green));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(20.0);

                    if self.token.is_some() {
                        if ui.button("Logout").clicked() {
                            self.logout();
                        }
                        ui.label(egui::RichText::new(self.email.as_ref().unwrap_or(&"".to_string())).color(egui::Color32::GRAY));

                        let history_btn_text = if self.show_history { "Hide History" } else { "Show History" };
                        if ui.button(history_btn_text).clicked() {
                            self.show_history = !self.show_history;
                        }
                    } else {
                        if ui.button("Login").clicked() {
                            self.view = View::Login;
                        }
                    }
                });
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.add_space(20.0);

                egui::ComboBox::from_id_source("method")
                    .selected_text(egui::RichText::new(self.method.as_str()).color(green))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.method, Method::GET, "GET");
                        ui.selectable_value(&mut self.method, Method::POST, "POST");
                        ui.selectable_value(&mut self.method, Method::PUT, "PUT");
                        ui.selectable_value(&mut self.method, Method::DELETE, "DELETE");
                        ui.selectable_value(&mut self.method, Method::PATCH, "PATCH");
                    });

                let url_response = ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .hint_text("Enter URL... (Cmd+L to focus)")
                        .desired_width(ui.available_width() - 100.0)
                );

                if self.url_field_focused {
                    url_response.request_focus();
                    self.url_field_focused = false;
                }

                let send_btn = egui::Button::new(
                    egui::RichText::new("Send").color(egui::Color32::BLACK)
                ).fill(green);

                if ui.add(send_btn).clicked() && !self.url.is_empty() {
                    self.send_request();
                }

                ui.add_space(20.0);
            });

            if self.method == Method::POST || self.method == Method::PUT || self.method == Method::PATCH {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label("Body:");
                });
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let body_field = egui::TextEdit::multiline(&mut self.body)
                        .hint_text("JSON body...")
                        .desired_width(ui.available_width() - 40.0)
                        .desired_rows(4);
                    ui.add(body_field);
                    ui.add_space(20.0);
                });
            }

            ui.add_space(20.0);

            if !self.status.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let status_color = if self.status.starts_with("2") {
                        green
                    } else if self.status.starts_with("4") {
                        egui::Color32::from_rgb(255, 180, 0)
                    } else {
                        egui::Color32::from_rgb(255, 80, 80)
                    };
                    ui.label(egui::RichText::new(&format!("Status: {}", self.status)).color(status_color));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new(&self.time).color(egui::Color32::GRAY));
                });
            }

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label("Response:");
            });

            egui::ScrollArea::vertical().max_height(ui.available_height() - 20.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let mut response_clone = self.response.clone();
                    let response_field = egui::TextEdit::multiline(&mut response_clone)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width() - 40.0);
                    ui.add(response_field);
                    ui.add_space(20.0);
                });
            });
        });
    }

    fn show_history_panel(&mut self, ui: &mut egui::Ui, green: egui::Color32) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("History").size(18.0).color(green));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(format!("{} requests", self.history.len())).size(12.0).color(egui::Color32::GRAY));
            });
        });
        ui.add_space(5.0);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut to_delete = None;
            let mut to_load = None;

            for item in &self.history {
                let is_selected = self.selected_history_id.as_ref() == item.id.as_ref();

                let frame = egui::Frame::none()
                    .fill(if is_selected { egui::Color32::from_rgb(30, 30, 30) } else { egui::Color32::from_rgb(18, 18, 18) })
                    .inner_margin(egui::Margin::same(10.0));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let method_color = match item.method.as_str() {
                            "GET" => egui::Color32::from_rgb(100, 180, 255),
                            "POST" => green,
                            "PUT" => egui::Color32::from_rgb(255, 180, 100),
                            "DELETE" => egui::Color32::from_rgb(255, 100, 100),
                            "PATCH" => egui::Color32::from_rgb(200, 150, 255),
                            _ => egui::Color32::GRAY,
                        };

                        ui.label(egui::RichText::new(&item.method).color(method_color).strong());

                        let url_display = if item.url.len() > 35 {
                            format!("{}...", &item.url[..35])
                        } else {
                            item.url.clone()
                        };

                        if ui.selectable_label(false, url_display).clicked() {
                            to_load = Some(item.clone());
                        }
                    });

                    ui.horizontal(|ui| {
                        let status_color = if item.status.starts_with("2") {
                            green
                        } else if item.status.starts_with("4") {
                            egui::Color32::from_rgb(255, 180, 0)
                        } else {
                            egui::Color32::from_rgb(255, 80, 80)
                        };

                        ui.label(egui::RichText::new(&item.status).size(11.0).color(status_color));
                        ui.label(egui::RichText::new(&item.time).size(11.0).color(egui::Color32::GRAY));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("×").clicked() {
                                if let Some(ref id) = item.id {
                                    to_delete = Some(id.clone());
                                }
                            }
                        });
                    });
                });

                ui.add_space(2.0);
            }

            if let Some(ref id) = to_delete {
                self.delete_history_item(id);
            }

            if let Some(item) = to_load {
                self.load_history_item(&item);
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Cmd+H: Toggle • Cmd+L: URL • Cmd+Enter: Send").size(10.0).color(egui::Color32::DARK_GRAY));
        });
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated = &s[..max_len];
        format!("{}... [truncated {} bytes]", truncated, s.len() - max_len)
    }
}

fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("restty");
    fs::create_dir_all(&path).ok();
    path.push("credentials.json");
    path
}

fn save_credentials(token: &str, user_id: &str, email: &str) {
    let creds = serde_json::json!({
        "token": token,
        "user_id": user_id,
        "email": email,
    });

    if let Ok(json) = serde_json::to_string_pretty(&creds) {
        fs::write(get_config_path(), json).ok();
    }
}

fn load_credentials() -> (Option<String>, Option<String>, Option<String>) {
    if let Ok(data) = fs::read_to_string(get_config_path()) {
        if let Ok(creds) = serde_json::from_str::<Value>(&data) {
            let token = creds["token"].as_str().map(String::from);
            let user_id = creds["user_id"].as_str().map(String::from);
            let email = creds["email"].as_str().map(String::from);
            return (token, user_id, email);
        }
    }
    (None, None, None)
}

fn clear_credentials() {
    fs::remove_file(get_config_path()).ok();
}

mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        if cfg!(target_os = "macos") {
            std::env::var("HOME").ok().map(|home| {
                PathBuf::from(home).join("Library").join("Application Support")
            })
        } else if cfg!(target_os = "linux") {
            std::env::var("HOME").ok().map(|home| {
                PathBuf::from(home).join(".config")
            })
        } else {
            None
        }
    }
}
