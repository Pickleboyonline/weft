//! What the checker knows about the standard library and the platforms: names,
//! tags, markup, and which built-in calls have which effects.

use crate::effects::Effect;

/// Lowercase names available everywhere without a declaration.
pub const FUNCTIONS: &[&str] = &[
    "now", "today", "new_id", "random", "sleep", "go", "status", "all", "race", "daily", "weekly", "cron",
];

/// Platform modules: `http.get(…)`.
pub const MODULES: &[&str] = &["http", "ws", "dom", "browser", "secrets"];

/// Markup names that aren't elements with attributes.
pub const MARKUP_CONSTANTS: &[&str] = &["nothing", "outlet"];

/// HTML elements in snake_case, plus weft's additions.
pub const ELEMENTS: &[&str] = &[
    "main", "nav", "section", "header", "footer", "article", "aside", "div", "h1", "h2", "h3", "h4", "h5", "h6", "p",
    "span", "strong", "em", "code", "pre", "br", "hr", "ul", "ol", "li", "form", "input", "textarea", "select",
    "option", "label", "button", "img", "a", "checkbox", "link", "table", "thead", "tbody", "tr", "td", "th",
];

/// Type constructors and configuration types from the standard library.
pub const TYPES: &[&str] = &[
    "Int", "Dec", "Float", "Bool", "Text", "Bytes", "Time", "Date", "Duration", "Size", "Id", "List", "Dict", "Set",
    "Range", "Option", "Result", "Table", "Query", "Command", "Check", "Pending", "Element", "Secret", "Action",
    "Html", "Route", "Deployment", "Machine", "Simulation", "Network", "Crashes", "Outside", "Extern", "Job",
    "Endpoint", "Request", "Response", "Fallible", "Constant", "Script", "Socket",
];

/// Wrappers whose argument is a function with a special role.
pub const ROLE_WRAPPERS: &[&str] = &["Query", "Command", "Check"];

/// Methods on lists (and on table reads) that call their function argument
/// immediately, so its effects count where the call is.
pub const IMMEDIATE_METHODS: &[&str] = &[
    "map", "filter", "flat_map", "fold", "count_if", "any", "all", "find", "sort_by", "distinct_by", "group_by",
    "with_default", "ok_or", "map_err", "on_ok", "on_err", "update", "update_where", "delete_where", "for_each",
];

/// Table methods that read.
pub const TABLE_READS: &[&str] = &[
    "get", "has", "rows", "filter", "count", "count_if", "any", "all", "find", "map", "sort_by", "group_by", "take",
    "skip", "first", "last", "is_empty", "contains", "sum", "min", "max", "distinct", "distinct_by", "flat_map",
    "fold", "reverse",
];

/// Table methods that write.
pub const TABLE_WRITES: &[&str] = &["insert", "update", "delete", "update_where", "delete_where"];

/// Methods that work inside a query on a table (reference: Standard library,
/// "Query methods"), beyond the table methods themselves.
pub const QUERY_METHODS: &[&str] = &[
    "abs", "min", "max", "round", "floor", "ceil", "to_int", "to_dec", "to_float", "length", "is_empty", "trim",
    "trim_start", "trim_end", "lower", "upper", "contains", "starts_with", "ends_with", "concat", "slice", "date",
    "day", "month", "year", "with_default", "is_some", "is_none", "map", "count", "count_if", "first", "find", "any",
    "all", "filter", "flat_map", "sum", "sort_by", "reverse", "take", "skip", "distinct", "distinct_by", "group_by",
    "get", "has",
];

/// Effects of calling a built-in function by name.
pub fn function_effect(name: &str) -> Option<Effect> {
    Some(match name {
        "now" | "today" => Effect::Time,
        "new_id" | "random" => Effect::Random,
        "sleep" => Effect::Wait,
        "go" => Effect::Navigate,
        "status" => Effect::Status,
        _ => return None,
    })
}

/// Effects of calling `module.function(…)`.
pub fn module_effect(module: &str, function: &str) -> Effect {
    match module {
        "secrets" => Effect::Secret,
        "dom" => Effect::Dom,
        "browser" => Effect::Browser,
        _ => Effect::Outside(format!("{module}.{function}")),
    }
}

/// Methods on platform handles (`Element`, sockets) and their effects.
pub fn handle_method_effect(method: &str) -> Option<Effect> {
    Some(match method {
        "focus" | "blur" | "size" | "scroll_into_view" | "canvas_2d" | "webgl" => Effect::Dom,
        "send" | "receive" | "close" => Effect::Outside(format!("socket.{method}")),
        _ => return None,
    })
}
