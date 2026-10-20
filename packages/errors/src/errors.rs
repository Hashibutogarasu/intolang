use serde::{Deserialize, Serialize};

use crate::catalog::CatalogEntry;
use crate::context::Severity;

/// Which side raised an error: the Into program or the toolchain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    IntoLang,
    Compiler,
}

macro_rules! declare_errors {
    (
        $(
            $domain:ident {
                $(
                    $(#[$meta:meta])*
                    $name:ident $({ $($field:ident : $fty:ty),+ $(,)? })? = $code:literal,
                )*
            }
        )*
    ) => {
        /// Every error of the toolchain with its code, message and domain.
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        #[repr(u16)]
        pub enum Errors {
            $($(
                $(#[$meta])*
                $name $({ $($field : $fty),+ })? = $code,
            )*)*
            #[error("Unknown error")]
            Unknown = 0xFFFF,
        }

        impl Errors {
            /// The code of the error; `Unknown` has none.
            pub const fn code(&self) -> Option<u16> {
                match self {
                    $($(Self::$name { .. } => Some($code),)*)*
                    Self::Unknown => None,
                }
            }

            pub const fn domain(&self) -> Domain {
                match self {
                    $($(Self::$name { .. } => Domain::$domain,)*)*
                    Self::Unknown => Domain::Compiler,
                }
            }
        }

        pub(crate) const ENTRIES: &[CatalogEntry] = &[
            $($(
                CatalogEntry {
                    code: $code,
                    domain: Domain::$domain,
                    name: stringify!($name),
                },
            )*)*
        ];
    };
}

impl Errors {
    pub const fn severity(&self) -> Severity {
        match self {
            Self::UnusedVariable { .. } | Self::UnusedImport { .. } => Severity::Hint,
            Self::NotOwning { .. } => Severity::Information,
            _ => Severity::Error,
        }
    }
}

declare_errors! {
    IntoLang {
        #[error("{message}")]
        Syntax { message: String } = 0x0001,
        #[error("`{text}` is not {expected}")]
        NotLexeme { text: String, expected: String } = 0x0002,
        #[error("Unsupported {kind} `{text}`")]
        Unsupported { kind: String, text: String } = 0x0003,
        #[error("Expected a type name")]
        ExpectedTypeName = 0x0004,
        #[error("{construct} needs {needs}")]
        TypeNeeds { construct: String, needs: String } = 0x0005,
        #[error("Type arguments need a type name before `<`")]
        TypeArgumentsNeedName = 0x0006,
        #[error("A character literal holds exactly one character: '{raw}'")]
        InvalidCharLiteral { raw: String } = 0x0007,
        #[error("Unterminated string at line {line}")]
        UnterminatedString { line: usize } = 0x0008,
        #[error("Unterminated character literal at line {line}")]
        UnterminatedChar { line: usize } = 0x0009,
        #[error("link attribute expects language, source path, and target function (line {line})")]
        LinkAttributeArguments { line: usize } = 0x000A,
        #[error("Native source path must remain inside the imported package: {path}")]
        NativeSourceOutside { path: String } = 0x000B,
        #[error("Link target function cannot be empty")]
        LinkTargetEmpty = 0x000C,
        #[error("`{name}` was moved")]
        Moved { name: String } = 0x2002,
        #[error("`{name}` was destroyed")]
        Destroyed { name: String } = 0x2003,
        #[error("`{name}` was detached")]
        Detached { name: String } = 0x2004,
        #[error("`{name}` is borrowed")]
        Borrowed { name: String } = 0x2005,
        #[error("`{receiver}` does not own its instance (`{method}` requires ownership)")]
        NotOwned { receiver: String, method: String } = 0x2006,
        #[error("returns a reference to a local (`{name}`)")]
        ReturnsLocalReference { name: String } = 0x2007,
        #[error("`{name}` is a reference; pass `{name}.copy()` where `{param}` expects an owned value")]
        ReferenceNeedsCopy { name: String, param: String } = 0x2008,
        #[error("`{value}` does not produce a value")]
        ProducesNoValue { value: String } = 0x2009,
        #[error("Unknown event `{event}`")]
        UnknownEvent { event: String } = 0x3001,
        #[error("`{event}.on` expects one lambda handler")]
        HandlerNeedsLambda { event: String } = 0x3003,
        #[error("The handler of `{event}` takes {expected} parameter(s) but the lambda declares {declared}")]
        HandlerParameterCount { event: String, expected: usize, declared: usize } = 0x3004,
        #[error("The event parameter `{param}` needs a type")]
        EventParameterNeedsType { param: String } = 0x3005,
        #[error("`{name}` takes {expected} argument(s) but received {received}")]
        ArgumentCount { name: String, expected: usize, received: usize } = 0x3006,
        #[error("Unknown event argument `{arg}`")]
        UnknownEventArgument { arg: String } = 0x3007,
        #[error("`{event}` expects `{expected}` for `{param}` but received `{actual}`")]
        EventArgumentType { event: String, expected: String, param: String, actual: String } = 0x3008,
        #[error("Recursive call to `{function}` is not supported")]
        RecursiveCall { function: String } = 0x1018,
        #[error("Function `{function}` expects {expected} argument(s) but received {received}")]
        FunctionArgumentCount { function: String, expected: usize, received: usize } = 0x1011,
        #[error("Missing required argument `{name}`")]
        MissingArgument { name: String } = 0x100F,
        #[error("Method `{function}` requires a receiver")]
        MethodNeedsReceiver { function: String } = 0x1019,
        #[error("Function `{function}` is not a method")]
        NotAMethod { function: String } = 0x101A,
        #[error("Unsupported argument `{argument}`")]
        UnsupportedArgument { argument: String } = 0x101B,
        #[error("Unsupported print argument `{argument}`")]
        UnsupportedPrintArgument { argument: String } = 0x101C,
        #[error("Unsupported statement in function body: `{statement}`")]
        UnsupportedStatement { statement: String } = 0x101D,
        #[error("Unsupported return value `{expression}`")]
        UnsupportedReturnValue { expression: String } = 0x1020,
        #[error("Unsupported if condition in function body: {reason}")]
        UnsupportedIfCondition { reason: String } = 0x1050,
        #[error("Undefined function `{key}`")]
        UndefinedFunction { key: String } = 0x1006,
        #[error("`{name}` takes {expected} parameter(s) but the lambda declares {declared}")]
        LambdaParameterCount { name: String, expected: usize, declared: usize } = 0x1012,
        #[error("The parameter `{param}` is a `{own}` but `{expected}` passes a `{passed}`")]
        LambdaParameterType { param: String, own: String, expected: String, passed: String } = 0x1014,
        #[error("A lambda inside a function cannot use the names of that function: {names}")]
        LambdaCapturesFunctionNames { names: String } = 0x1015,
        #[error("`{method}` gives a value that has to be used")]
        ValueMustBeUsed { method: String } = 0x1060,
        #[error("Expected a pointer but received `{value}`")]
        ExpectedPointer { value: String } = 0x1061,
        #[error("`await` expects a Task but received `{value}`")]
        AwaitExpectsTask { value: String } = 0x3010,
        #[error("`on` of a Signal expects a signal number and a lambda")]
        SignalOnArguments = 0x3020,
        #[error("The handler of a signal takes the signal number")]
        SignalHandlerParameter = 0x3021,
        #[error("Receiving signals needs the `{event}` event: import \"ps://events\"")]
        SignalEventMissing { event: String } = 0x3022,
        #[error("`{number}` is not a valid signal number")]
        InvalidSignalNumber { number: String } = 0x3023,
        #[error("The signal `{number}` must be a number or a constant such as Signal.SIGTERM")]
        SignalNotNumber { number: String } = 0x3024,
        #[error("The parameter `{param}` of `{function}` needs a type")]
        ParameterNeedsType { param: String, function: String } = 0x1070,
        #[error("A lambda cannot stand where a `{expected}` is expected")]
        LambdaNotAllowed { expected: String } = 0x1016,
        #[error("Expected a `{expected}` argument but received `{argument}`")]
        ExpectedArgumentType { expected: String, argument: String } = 0x100D,
        #[error("Expected a `{expected}` but received `{received}`")]
        ExpectedType { expected: String, received: String } = 0x100B,
        #[error("Expected a `{expected}` but received a `{received}`")]
        ExpectedTypeReceived { expected: String, received: String } = 0x100C,
        #[error("`return` needs a `{ty}` value")]
        ReturnNeedsValue { ty: String } = 0x1071,
        #[error("A function that returns nothing cannot return a value")]
        VoidReturnsValue = 0x1072,
        #[error("Arithmetic needs numbers but received `{left}` and `{right}`")]
        ArithmeticNeedsNumbers { left: String, right: String } = 0x1073,
        #[error("A lambda has no type here")]
        LambdaHasNoType = 0x1074,
        #[error("`await` has no value")]
        AwaitHasNoValue = 0x1075,
        #[error("Unknown name `{path}`")]
        UnknownName { path: String } = 0x1076,
        #[error("Unknown receiver `{name}`")]
        UnknownReceiver { name: String } = 0x1078,
        #[error("Arithmetic result type must be numeric")]
        ArithmeticResultNotNumeric = 0x1079,
        #[error("`{operator}` needs integers but received `{ty}`")]
        ArithmeticNeedsIntegers { operator: String, ty: String } = 0x107A,
        #[error("no cast from {from} to {to}")]
        NoCast { from: String, to: String } = 0x107B,
        #[error("Arithmetic requires a typed numeric value, not `{expression}`")]
        ArithmeticNeedsTypedValue { expression: String } = 0x107C,
        #[error("Arithmetic requires a typed numeric value")]
        ArithmeticNeedsTypedValueBare = 0x107D,
        #[error("Unsupported arithmetic operand `{operand}`")]
        UnsupportedArithmeticOperand { operand: String } = 0x107E,
        #[error("`{value}` is not a Boolean expression")]
        NotBoolean { value: String } = 0x107F,
        #[error("`{class}` has no field `{field}`")]
        NoField { class: String, field: String } = 0x1008,
        #[error("`{key}` is not available; import \"intolang://builtin/std\"")]
        NotAvailable { key: String } = 0x1080,
        #[error("`{value}` is a `{ty}`, not text")]
        NotText { value: String, ty: String } = 0x1081,
        #[error("Unsupported text operand `{operand}`")]
        UnsupportedTextOperand { operand: String } = 0x1082,
        #[error("Texts can only be compared with `==` and `!=`")]
        TextCompareOperator = 0x1083,
        #[error("A text can only be compared with a text, not `{side}`")]
        TextCompareOperand { side: String } = 0x1084,
        #[error("Static field `{field}` has no constant default")]
        StaticFieldNoDefault { field: String } = 0x1085,
        #[error("A lambda needs a callback type; annotate the binding, such as `Callback<String>`")]
        LambdaNeedsCallbackType = 0x1017,
        #[error("`await` can only be used as a statement")]
        AwaitOnlyStatement = 0x1086,
        #[error("`{value}` is not a typed value")]
        NotTypedValue { value: String } = 0x1087,
        #[error("`{callee}` does not return a value")]
        NoReturnValue { callee: String } = 0x1088,
        #[error("Function `{key}` does not return a value")]
        FunctionNoReturnValue { key: String } = 0x1089,
        #[error("Imported function `{function}` has no #[link] attribute")]
        ImportedFunctionNoLink { function: String } = 0x108A,
        #[error("Expected a `{element}` item but received `{item}`")]
        ExpectedItemType { element: String, item: String } = 0x100E,
        #[error("Unsupported list item `{item}`")]
        UnsupportedListItem { item: String } = 0x1021,
        #[error("An empty array needs a type, such as `[Int32]`")]
        EmptyArrayNeedsType = 0x108B,
        #[error("`forEach` takes 1 argument but received {received}")]
        ForEachArgumentCount { received: usize } = 0x108C,
        #[error("The lambda of `forEach` takes the item and optionally the index, not {count} parameter(s)")]
        ForEachLambdaParameters { count: usize } = 0x108D,
        #[error("The parameter `{param}` is a `{own}` but the items are `{element}`")]
        ForEachParameterType { param: String, own: String, element: String } = 0x108E,
        #[error("`forEach` expects a lambda or a function, not `{value}`")]
        ForEachAction { value: String } = 0x108F,
        #[error("Cannot instantiate the abstract class `{class}`")]
        AbstractClassInstantiated { class: String } = 0x4020,
        #[error("Type mismatch: expected `{expected}` but found `{found}`")]
        TypeMismatch { expected: String, found: String } = 0x100A,
        #[error("`get` of the dynamic value `{holder}` has no value")]
        DynamicGetNoValue { holder: String } = 0x1090,
        #[error("The class `Type` has no field `name`")]
        TypeClassNoName = 0x1091,
        #[error("Undefined cast handler `{key}`")]
        UndefinedCastHandler { key: String } = 0x1092,
        #[error("`{value}` cannot be cast")]
        CannotBeCast { value: String } = 0x1093,
        #[error("An object literal needs the class `Json`")]
        ObjectNeedsJson = 0x1094,
        #[error("`Json.object` has no value")]
        JsonObjectNoValue = 0x1095,
        #[error("The value `{value}` of the entry `{key}` cannot be stored in an object")]
        ObjectEntryNotStorable { value: String, key: String } = 0x1096,
        #[error("`{path}` is not a method call")]
        NotAMethodCall { path: String } = 0x1097,
        #[error("Undefined member `{member}` on `{ty}`")]
        UndefinedMember { member: String, ty: String } = 0x1098,
        #[error("{label} condition requires an integer or float value")]
        ConditionNeedsNumber { label: String } = 0x1099,
        #[error("{label} condition requires a numeric literal or value")]
        ConditionNeedsLiteral { label: String } = 0x109A,
        #[error("Unsupported native argument `{arg}`")]
        UnsupportedNativeArgument { arg: String } = 0x109B,
        #[error("Undefined enum variant `{arg}`")]
        UndefinedEnumVariant { arg: String } = 0x109C,
        #[error("Function `{function}` is not imported")]
        FunctionNotImported { function: String } = 0x1077,
        #[error("Function `{function}` has no native link")]
        FunctionNoNativeLink { function: String } = 0x10A0,
        #[error("{label} condition `{callee}` must return Boolean")]
        ConditionMustReturnBoolean { label: String, callee: String } = 0x10A1,
        #[error("{label} condition: {reason}")]
        ConditionFailed { label: String, reason: String } = 0x10A2,
        #[error("Only a name or a field of a name can be assigned, not `{target}`")]
        AssignTargetInvalid { target: String } = 0x10A3,
        #[error("Undefined name `{name}`")]
        UndefinedName { name: String } = 0x1002,
        #[error("`{holder}` holds no instance")]
        HoldsNoInstance { holder: String } = 0x10A4,
        #[error("The field `{field}` is a `{expected}` but `{name}` holds no such value")]
        FieldHoldsNoValue { field: String, expected: String, name: String } = 0x10A5,
        #[error("Assigning `{value}` to the `{ty}` field `{field}` is not supported")]
        FieldAssignUnsupported { value: String, ty: String, field: String } = 0x10A6,
        #[error("Undefined variable `{name}`")]
        UndefinedVariable { name: String } = 0x1003,
        #[error("Type `{class}` has no declared member fields")]
        NoDeclaredFields { class: String } = 0x10A7,
        #[error("`break` must be inside a while loop")]
        BreakOutsideLoop = 0x10A8,
        #[error("Native call `{symbol}` expects a string or 32-bit integer argument")]
        NativeCallArgument { symbol: String } = 0x10A9,
        #[error("The import `{uri}` names no package or module")]
        ImportNotFound { uri: String } = 0x10AA,
        #[error("`{name}` is a constant and cannot be assigned again")]
        AssignToConstant { name: String } = 0x10AB,
        #[error("`{name}` is never used")]
        UnusedVariable { name: String } = 0x10AC,
        #[error("The import `{uri}` is never used")]
        UnusedImport { uri: String } = 0x10AD,
        #[error("`{name}` owns nothing: it is a reference")]
        NotOwning { name: String } = 0x10AE,
        #[error("Variable `{name}` is out of scope here")]
        VariableOutOfScope { name: String } = 0x1040,
        #[error("Variable `{name}` was unloaded")]
        VariableUnloaded { name: String } = 0x1041,
        #[error("Undefined method `{key}`")]
        UndefinedMethod { key: String } = 0x1004,
        #[error("Undefined static method `{key}`")]
        UndefinedStaticMethod { key: String } = 0x1005,
        #[error("Arithmetic type `{declared}` does not match operand type `{operand}`")]
        ArithmeticTypeMismatch { declared: String, operand: String } = 0x1030,
        #[error("No overload of `{key}` accepts ({given}); the choices are {choices}")]
        NoOverload { key: String, given: String, choices: String } = 0x1031,
        #[error("The field `{field}` is a `{expected}` but received `{received}`")]
        FieldTypeMismatch { field: String, expected: String, received: String } = 0x1032,
        #[error("The field `{field}` is a `{expected}` but received a `{received}`")]
        FieldReceivedType { field: String, expected: String, received: String } = 0x1033,
        #[error("Unknown class `{name}`")]
        UnknownClass { name: String } = 0x4001,
        #[error("Class `{name}` inherits from itself")]
        ClassInheritsItself { name: String } = 0x4002,
        #[error("`Object` is a built-in class")]
        ObjectIsBuiltin = 0x4003,
        #[error("Duplicate {noun} `{name}`")]
        DuplicateMember { noun: String, name: String } = 0x4004,
        #[error("Abstract member `{name}` must be declared in an abstract class")]
        AbstractMemberOutsideAbstractClass { name: String } = 0x4005,
        #[error("Abstract member `{name}` cannot have a body")]
        AbstractMemberHasBody { name: String } = 0x4006,
        #[error("`{name}` is marked `override` but overrides nothing")]
        OverrideOverridesNothing { name: String } = 0x4007,
        #[error("`{name}` hides a member of `{ancestor}`; add `override`")]
        MemberHidesInherited { name: String, ancestor: String } = 0x4008,
        #[error("`{name}` of `{ancestor}` is final and cannot be redeclared")]
        MemberFinalRedeclared { name: String, ancestor: String } = 0x4009,
        #[error("`{name}` of `{ancestor}` is final and cannot be overridden")]
        MemberFinalOverridden { name: String, ancestor: String } = 0x400A,
        #[error("`{name}` does not match the signature of `{ancestor}.{name}`")]
        SignatureMismatch { name: String, ancestor: String } = 0x400B,
        #[error("Class `{class}` does not implement the abstract member `{name}`")]
        AbstractMemberNotImplemented { class: String, name: String } = 0x400C,
        #[error("Enum names must start with an uppercase letter")]
        EnumNameCase = 0x4010,
        #[error("Enum discriminants must be 32-bit integers")]
        EnumDiscriminantRange = 0x4011,
        #[error("Enum variant names must start with an uppercase letter")]
        EnumVariantNameCase = 0x4012,
        #[error("Duplicate enum variant `{name}`")]
        DuplicateEnumVariant { name: String } = 0x4013,
        #[error("Duplicate enum `{name}`")]
        DuplicateEnum { name: String } = 0x4014,
        #[error("{path}: test module must import `{function}`")]
        TestMustImportAssertion { path: String, function: String } = 0x7001,
        #[error("{path}: test module must import the test macro")]
        TestMustImportMacro { path: String } = 0x7002,
        #[error("{path}: no #[test] functions found")]
        NoTestFunctions { path: String } = 0x7003,
        #[error("test macro does not define the #[test] annotation")]
        TestMacroNoAnnotation = 0x7004,
        #[error("{path}: no function `{name}` to run as a test")]
        NoTestFunction { path: String, name: String } = 0x7005,
        #[error("Into assertion failed: {message}")]
        AssertionFailed { message: String } = 0x6002,
        #[error("Every thread is waiting: deadlock")]
        ThreadsDeadlock = 0x6003,
        #[error("Waiting for process {pid}, which never exits: deadlock")]
        ProcessDeadlock { pid: String } = 0x6004,
        #[error("Division by zero")]
        DivisionByZero = 0x6005,
        #[error("Stack overflow in `{function}`")]
        StackOverflow { function: String } = 0x6006,
        #[error("Stack overflow in a call of `{slot}`")]
        StackOverflowIndirect { slot: String } = 0x6007,
        #[error("`{slot}` does not refer to a function")]
        NotAFunction { slot: String } = 0x6008,
        #[error("`{slot}` refers to the unknown function `{name}`")]
        UnknownFunctionReference { slot: String, name: String } = 0x6009,
        #[error("`{slot}` holds no instance")]
        NoInstance { slot: String } = 0x600A,
        #[error("`{name}` has no binding")]
        NoBinding { name: String } = 0x600B,
        #[error("The value holds no instance")]
        ValueNoInstance = 0x600C,
        #[error("Arithmetic needs numbers")]
        ArithmeticNeedsNumbersVm = 0x600D,
        #[error("Values of different kinds cannot be compared")]
        KindsNotComparable = 0x600E,
        #[error("A value of type `{ty}` cannot be printed")]
        NotPrintable { ty: String } = 0x600F,
        #[error("`{task}` holds no thread")]
        NoThread { task: String } = 0x6010,
        #[error("`{symbol}` takes more than {max} arguments")]
        TooManyNativeArguments { symbol: String, max: usize } = 0x6011,
        #[error("Floating point arguments are not supported in native calls")]
        FloatArgument = 0x6012,
        #[error("{value} is not a pointer")]
        NotAPointer { value: String } = 0x6013,
        #[error("An item of the type `{ty}` cannot be read from memory")]
        ItemNotReadable { ty: String } = 0x6014,
        #[error("`{pointer}` is a null pointer")]
        NullPointer { pointer: String } = 0x6015,
        #[error("The index is not a number")]
        IndexNotNumber = 0x6016,
        #[error("{value} is not a number")]
        NotANumber { value: String } = 0x6017,
        #[error("{value} cannot be written as an item of type `{ty}`")]
        ItemNotWritable { value: String, ty: String } = 0x6018,
        #[error("{value} is not text")]
        NotTextValue { value: String } = 0x6019,
        #[error("{value} is not a list")]
        NotAList { value: String } = 0x601A,
        #[error("The list call is missing its index")]
        ListMissingIndex = 0x601B,
        #[error("The list call is missing its item")]
        ListMissingItem = 0x601C,
        #[error("The list was freed")]
        ListFreed = 0x601D,
        #[error("`asset` takes a path")]
        AssetNeedsPath = 0x601E,
        #[error("`Assets.read` takes the path of a file")]
        AssetsReadNeedsPath = 0x601F,
        #[error("The program has no assets; call `require` first")]
        NoAssets = 0x6020,
        #[error("The asset `{path}` is not in the bundle")]
        AssetNotInBundle { path: String } = 0x6021,
        #[error("{value} is not a path")]
        NotAPath { value: String } = 0x6022,
        #[error("{value} is not a process handle")]
        NotProcessHandle { value: String } = 0x6023,
        #[error("The value is not a process handle")]
        NotProcessHandleBare = 0x6024,
        #[error("{value} is not an integer")]
        NotAnInteger { value: String } = 0x6025,
        #[error("The native call is missing argument {index}")]
        MissingNativeArgument { index: String } = 0x6026,
        #[error("The native call is missing the integer argument {index}")]
        MissingIntegerArgument { index: String } = 0x6027,
        #[error("The native function `{symbol}` is not available in the VM")]
        NativeFunctionUnavailable { symbol: String } = 0x6028,
        #[error("No running frame")]
        NoFrame = 0x6029,
        #[error("Dangling reference {reference}")]
        DanglingReference { reference: String } = 0x602A,
        #[error("Unknown field index {index}")]
        UnknownFieldIndexVm { index: String } = 0x602B,
        #[error("Unknown field index {index} of `{class}`")]
        UnknownFieldIndexOf { index: String, class: String } = 0x602C,
    }
    Compiler {
        #[error("`{text}` is not a package hash")]
        PackageHashInvalid { text: String } = 0x8002,
        #[error("Invalid package alias `{text}`")]
        PackageAliasInvalid { text: String } = 0x8003,
        #[error("Could not find project manifest: {path}: {reason}")]
        ManifestNotFound { path: String, reason: String } = 0x8004,
        #[error("Invalid project manifest: {path}: {reason}")]
        ManifestInvalid { path: String, reason: String } = 0x8005,
        #[error("A project manifest must be a JSON object")]
        ManifestNotObject = 0x8006,
        #[error("Project manifest `{field}` must be an object")]
        ManifestFieldNotObject { field: String } = 0x8007,
        #[error("Could not find package directory: {path}: {reason}")]
        PackageDirectoryNotFound { path: String, reason: String } = 0x8008,
        #[error("Invalid package manifest: {path}: {reason}")]
        PackageManifestInvalid { path: String, reason: String } = 0x8009,
        #[error("Package manifest must define `{key}`")]
        PackageManifestMissing { key: String } = 0x800A,
        #[error("The compiler does not embed the `{alias}` package")]
        PackageNotEmbedded { alias: String } = 0x800B,
        #[error("Package `{alias}` is not installed")]
        PackageNotInstalled { alias: String } = 0x800C,
        #[error("No {file} found; install a package with `into install <package>` first")]
        LockMissing { file: String } = 0x800D,
        #[error("Lock entry for `{alias}` has no location")]
        LockEntryNoLocation { alias: String } = 0x800E,
        #[error("Lock entry for `{alias}` has no hash")]
        LockEntryNoHash { alias: String } = 0x800F,
        #[error("Package `{alias}` is locked at version {locked} but the compiler provides version {provided}")]
        PackageVersionLocked { alias: String, locked: String, provided: String } = 0x8010,
        #[error("Could not find package `{alias}` at {path}: {reason}")]
        PackageLocationMissing { alias: String, path: String, reason: String } = 0x8011,
        #[error("Package `{alias}` at {path} does not match the lock file (expected {expected}, found {found})")]
        PackageHashMismatch { alias: String, path: String, expected: String, found: String } = 0x8012,
        #[error("Could not find project directory: {path}: {reason}")]
        ProjectDirectoryNotFound { path: String, reason: String } = 0x8013,
        #[error("Invalid export path in into-info.into: {path}")]
        ExportPathInvalid { path: String } = 0x8014,
        #[error("Package `{alias}` has no into-info.into")]
        PackageNoInfo { alias: String } = 0x8015,
        #[error("Package `{alias}` does not export module `{module}`")]
        ModuleNotExported { alias: String, module: String } = 0x8016,
        #[error("Unsupported lock file version {version}")]
        LockVersionUnsupported { version: u32 } = 0x8017,
        #[error("Invalid lock file: {path}: {reason}")]
        LockInvalid { path: String, reason: String } = 0x8018,
        #[error("`{uri}` is not a module URI")]
        NotModuleUri { uri: String } = 0x8019,
        #[error("`{uri}` does not name an export")]
        ExportNameMissing { uri: String } = 0x801A,
        #[error("`{uri}` does not name a module")]
        ImportNameMissing { uri: String } = 0x801B,
        #[error("Library entry has no parent")]
        LibraryNoParent = 0x801C,
        #[error("Library entry exports no public functions")]
        LibraryNoPublicFunctions = 0x801D,
        #[error("Invalid library export URI: {uri}")]
        LibraryExportUriInvalid { uri: String } = 0x801E,
        #[error("Invalid library export module: {uri}")]
        LibraryExportModuleInvalid { uri: String } = 0x801F,
        #[error("Library entry does not export any modules")]
        LibraryNoExports = 0x8020,
        #[error("Malformed {kind} record")]
        MalformedRecord { kind: String } = 0x9002,
        #[error("Malformed function parameter `{field}`")]
        MalformedParameter { field: String } = 0x9003,
        #[error("Unknown binding property `{name}`")]
        UnknownProperty { name: String } = 0x9004,
        #[error("Malformed native field argument `{text}`")]
        MalformedFieldArgument { text: String } = 0x9005,
        #[error("Unsupported native link argument type `{kind}`")]
        UnsupportedLinkArgument { kind: String } = 0x9006,
        #[error("Unknown storage `{name}`")]
        UnknownStorage { name: String } = 0x9007,
        #[error("Unknown ownership change `{name}`")]
        UnknownOwnershipChange { name: String } = 0x9008,
        #[error("Native source {path} is outside its package")]
        NativeSourceOutsidePackage { path: String } = 0x9009,
        #[error("Linked native source does not exist: {path}")]
        NativeSourceMissing { path: String } = 0x900A,
        #[error("pkg-config returned no library directory for `{library}`")]
        PkgConfigNoDirectory { library: String } = 0x900B,
        #[error("Could not run {tool}: {reason}")]
        ToolNotRun { tool: String, reason: String } = 0x900C,
        #[error("pkg-config failed for native dependencies {dependencies}")]
        PkgConfigFailed { dependencies: String } = 0x900D,
        #[error("Native module compilation failed for {path}")]
        NativeModuleCompilationFailed { path: String } = 0x900E,
        #[error("Native source compilation failed for {path}")]
        NativeSourceCompilationFailed { path: String } = 0x900F,
        #[error("Linking the native library failed")]
        NativeLibraryLinkFailed = 0x9010,
        #[error("Embedding the bundle failed")]
        BundleEmbedFailed = 0x9011,
        #[error("Unknown native class `{class}`")]
        UnknownNativeClass { class: String } = 0xA002,
        #[error("Unknown field index for class `{class}`")]
        UnknownFieldIndex { class: String } = 0xA003,
        #[error("Unknown event handler `{handler}`")]
        UnknownEventHandler { handler: String } = 0xA004,
        #[error("if end has no matching if")]
        IfEndWithoutIf = 0xA005,
        #[error("`else` has no matching if")]
        ElseWithoutIf = 0xA006,
        #[error("An if block cannot contain multiple else branches")]
        MultipleElse = 0xA007,
        #[error("Native value `{name}` has no storage")]
        NativeValueNoStorage { name: String } = 0xA008,
        #[error("`throw` can only run in the Into VM ({message})")]
        ThrowNeedsVm { message: String } = 0xA009,
        #[error("Cannot cast {from} to {to}")]
        CannotCast { from: String, to: String } = 0xA00A,
        #[error("`{slot}.{property}` can only be read by code that runs in the Into VM")]
        IfStateNeedsVm { slot: String, property: String } = 0xA00B,
        #[error("An index cannot be text")]
        IndexIsText = 0xA00C,
        #[error("Text cannot be stored as an item")]
        TextStored = 0xA00D,
        #[error("A printed constant must be a number")]
        PrintedConstantNotNumber = 0xA00E,
        #[error("Native break has no matching loop")]
        BreakWithoutLoop = 0xA00F,
        #[error("Native loop end has no matching loop")]
        LoopEndWithoutLoop = 0xA010,
        #[error("Call of the undefined function `{name}`")]
        UndefinedFunctionCall { name: String } = 0xA011,
        #[error("Reference to the undefined function `{name}`")]
        UndefinedFunctionReference { name: String } = 0xA012,
        #[error("Arithmetic cannot use a string operand")]
        ArithmeticStringOperand = 0xA013,
        #[error("Unsupported arithmetic operation `{operation}`")]
        UnsupportedArithmeticOperation { operation: String } = 0xA014,
        #[error("Native if condition must return Boolean")]
        IfConditionNotBoolean = 0xA015,
        #[error("Native loop condition must return Boolean")]
        LoopConditionNotBoolean = 0xA016,
        #[error("DummyProcess requires the Into VM (`{target}`)")]
        DummyProcessNeedsVm { target: String } = 0xA017,
        #[error("Malformed handler parameter `{field}`")]
        MalformedHandlerParameter { field: String } = 0xA018,
        #[error("The defaults of `{class}` build instances without end")]
        InstancesWithoutEnd { class: String } = 0xA019,
        #[error("Unknown thread function `{function}`")]
        UnknownThreadFunction { function: String } = 0xA01A,
        #[error("Captured value `{name}` has no storage")]
        CapturedValueNoStorage { name: String } = 0xA01B,
        #[error("Handler end has no matching handler")]
        HandlerEndWithoutHandler = 0xA01C,
        #[error("A comparison cannot use a string operand")]
        ComparisonStringOperand = 0xA01D,
        #[error("Class `{class}` has no field {index}")]
        ClassNoFieldIndex { class: String, index: usize } = 0xA01E,
        #[error("Native pointer argument `{name}` has no value")]
        PointerArgumentNoValue { name: String } = 0xA01F,
        #[error("{context}")]
        LlvmFailed { context: String } = 0xA020,
        #[error("LLD link failed ({status})")]
        LldLinkFailed { status: String } = 0xA021,
        #[error("{message}")]
        Io { message: String } = 0xB001,
        #[error("{message}")]
        Json { message: String } = 0xB005,
        #[error("Could not read {path}: {reason}")]
        Read { path: String, reason: String } = 0xB002,
        #[error("Could not write {path}: {reason}")]
        Write { path: String, reason: String } = 0xB003,
        #[error("Could not {action} {path}: {reason}")]
        FileAction { action: String, path: String, reason: String } = 0xB004,
        #[error("A bundle path cannot be empty")]
        BundlePathEmpty = 0xB101,
        #[error("`{path}` is absolute; a bundle path is relative")]
        BundlePathAbsolute { path: String } = 0xB102,
        #[error("`{path}` holds a character a bundle path cannot hold")]
        BundlePathCharacter { path: String } = 0xB103,
        #[error("`{path}` has an empty component")]
        BundlePathEmptyComponent { path: String } = 0xB104,
        #[error("`{path}` has the component `{component}`")]
        BundlePathDotComponent { path: String, component: String } = 0xB105,
        #[error("`{path}` is already in the bundle")]
        BundleDuplicate { path: String } = 0xB106,
        #[error("`{path}` is a file, not a directory")]
        BundleNotDirectory { path: String } = 0xB107,
        #[error("`{path}` nests too deep")]
        BundleTooDeep { path: String } = 0xB108,
        #[error("{message}")]
        Internal { message: String } = 0xC001,
        #[error("{message}")]
        Panic { message: String } = 0xC002,
        #[error("Unknown message tag `{tag}`")]
        UnknownMessageTag { tag: String } = 0xC003,
        #[error("Not a directive line")]
        NotDirectiveLine = 0xC004,
        #[error("Invalid native string argument encoding")]
        InvalidHexEncoding = 0xC005,
        #[error("Invalid number: {reason}")]
        InvalidNumber { reason: String } = 0xC006,
        #[error("Invalid native string: {reason}")]
        InvalidCString { reason: String } = 0xC007,
        #[error("Builtin package is not bundled")]
        BuiltinNotBundled = 0xC108,
        #[error("The message ends inside an unfinished block")]
        MessageUnfinishedBlock = 0xC101,
        #[error("{record} has no matching {expected}")]
        NoMatching { record: String, expected: String } = 0xC102,
        #[error("NATIVE_BREAK is outside a loop")]
        RecordBreakOutsideLoop = 0xC103,
        #[error("`{symbol}` is not a symbol")]
        NotASymbol { symbol: String } = 0xC104,
        #[error("Could not load the native library {path}: {reason}")]
        NativeLibraryLoad { path: String, reason: String } = 0xC106,
        #[error("The inex file is truncated")]
        InexTruncated = 0xB201,
        #[error("The inex file has a name that is not text")]
        InexNameNotText = 0xB202,
        #[error("Not an inex file")]
        NotInex = 0xB203,
        #[error("inex version {version} is not supported")]
        InexVersionUnsupported { version: String } = 0xB204,
        #[error("The inex file is truncated or has the entry `{path}` outside its data")]
        InexEntryOutside { path: String } = 0xB206,
        #[error("The hash of `{path}` does not match its contents")]
        InexHashMismatch { path: String } = 0xB207,
        #[error("The inex file has an entry of the unknown kind {kind}")]
        InexUnknownKind { kind: String } = 0xB208,
        #[error("inex has no intoc")]
        InexNoIntoc = 0xB209,
        #[error("The intoc of the inex is not text")]
        IntocNotText = 0xB20A,
        #[error("Could not find package manifest: {path}: {reason}")]
        PackageManifestNotFound { path: String, reason: String } = 0xB301,
        #[error("Manifest has no parent directory")]
        ManifestNoParent = 0xB302,
        #[error("Package manifest must specify exactly one of `runs` or `library`")]
        ManifestRunsOrLibrary = 0xB303,
        #[error("`--vm` builds executables; a library package has no .inex form")]
        VmLibraryUnsupported = 0xB304,
        #[error("Library packages require src/main/lib.into as their library entry point")]
        LibraryEntryInvalid = 0xB305,
        #[error("No Into test files found under {path}")]
        NoTestFiles { path: String } = 0xB306,
        #[error("{count} Into test(s) failed")]
        TestsFailed { count: usize } = 0xB307,
        #[error("Invalid installed package alias `{alias}`")]
        InstalledAliasInvalid { alias: String } = 0xB308,
        #[error("Package `{alias}` is declared as version {declared} but version {installed} is installed")]
        InstalledVersionMismatch { alias: String, declared: String, installed: String } = 0xB309,
        #[error("Program exited with status {status}")]
        ProgramFailed { status: String } = 0xB30A,
        #[error("No .into files found under {path}")]
        NoIntoFiles { path: String } = 0xB30B,
        #[error("Format mode expects an .into file or directory path")]
        FormatExpectsInto = 0xB30C,
        #[error("Project path must end with a valid project name")]
        ProjectNameInvalid = 0xB30D,
        #[error("Project path must name a new directory")]
        ProjectPathNotNew = 0xB30E,
        #[error("Project destination already exists: {path}")]
        ProjectExists { path: String } = 0xB30F,
        #[error("`{path}` is named to be bundled, but the package lacks it")]
        BundleResourceMissing { path: String } = 0xB310,
    }
}
