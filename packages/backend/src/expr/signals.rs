use ast::{Expr, Path, Stmt, StmtKind};
use std::collections::HashSet;
use types::{ComparisonOperator, Literal, MessageTag, Type};

use super::events::RuntimeCall;
use super::type_name::{base_type_name, parse_type};
use crate::context::Lowerer;

const SIGNAL: &str = "Signal";
const ON: &str = "on";
const WATCH: &str = "Signal.watch";
const SIGNAL_EVENT: &str = "signalReceived";

impl Lowerer<'_> {
    /// Lower `signal.on(number, handler)`: the handler joins the
    /// `signalReceived` event and only runs for the signal `number`. The
    /// signal is watched so that it reaches the event when it is dispatched.
    pub fn lower_signal_call(
        &mut self,
        callee: &Path,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<RuntimeCall, String> {
        if callee.last() != ON || self.chain_type(callee).is_none_or(|ty| !is_signal(&ty)) {
            return Ok(RuntimeCall::NotRuntime);
        }
        self.ensure_usable(callee.first())?;
        let [number, Expr::Lambda { params, body }] = args else {
            return Err("`on` of a Signal expects a signal number and a lambda".to_owned());
        };
        let [param] = params.as_slice() else {
            return Err("The handler of a signal takes the signal number".to_owned());
        };
        if !self.env.events.contains_key(SIGNAL_EVENT) {
            return Err(format!(
                "Receiving signals needs the `{SIGNAL_EVENT}` event: import \"ps://events\""
            ));
        }
        if self.state.destroyed_events.contains(SIGNAL_EVENT) {
            return Err(format!("`{SIGNAL_EVENT}` was destroyed"));
        }
        let number = self.signal_number(number)?;
        let only_this_signal = Stmt {
            line: 0,
            kind: StmtKind::If {
                condition: Expr::Compare {
                    left: Box::new(Expr::ident(param.name.clone())),
                    op: ComparisonOperator::Equal,
                    right: Box::new(Expr::int(i64::from(number))),
                },
                then_body: body.clone(),
                else_body: None,
            },
        };
        let handler = Expr::Lambda {
            params: params.clone(),
            body: vec![only_this_signal],
        };
        self.register_handler(SIGNAL_EVENT, &[handler], assigned_to)?;
        if let Some(link) = self.env.native_links.get(WATCH) {
            self.emit_link_call(
                MessageTag::LinkAssign,
                link,
                vec![Expr::int(i64::from(number))],
                "",
                false,
            )?;
        }
        Ok(RuntimeCall::Listener(SIGNAL_EVENT.to_owned()))
    }

    /// The type a member chain such as `process.signal` evaluates to, without
    /// lowering it.
    fn chain_type(&self, path: &Path) -> Option<Type> {
        let segments = path.segments();
        let mut current = self.state.values.get(&segments[0])?.ty.clone();
        for segment in &segments[1..segments.len().checked_sub(1)?] {
            let key = self.method_key(base_type_name(&current.name()), segment);
            let link = self.env.native_links.get(&key)?;
            current = parse_type(&link.return_type, &HashSet::new()).ok()?;
        }
        Some(current)
    }

    /// The signal number a literal or a static constant such as
    /// `Signal.SIGTERM` stands for.
    fn signal_number(&self, number: &Expr) -> Result<i32, String> {
        let literal = match number {
            Expr::Literal(literal) => Some(literal),
            Expr::Path(path) => self
                .static_field(path)
                .and_then(|field| field.default_literal()),
            _ => None,
        };
        match literal {
            Some(Literal::Int(value)) => i32::try_from(*value)
                .map_err(|_| format!("`{number}` is not a valid signal number")),
            _ => Err(format!(
                "The signal `{number}` must be a number or a constant such as Signal.SIGTERM"
            )),
        }
    }
}

fn is_signal(ty: &Type) -> bool {
    base_type_name(&ty.name()) == SIGNAL
}
