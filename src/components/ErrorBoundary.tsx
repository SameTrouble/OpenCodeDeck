import { Component, type ErrorInfo, type ReactNode } from "react"

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  message: string
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, message: "" }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, message: error.message || String(error) }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info)
  }

  reset = () => this.setState({ hasError: false, message: "" })

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen flex-col items-center justify-center gap-4 p-6 text-center">
          <h1 className="text-lg font-semibold">出错了</h1>
          <p className="text-sm text-muted-foreground">应用遇到错误，请尝试重置或重启应用。</p>
          <pre className="max-w-md overflow-auto rounded bg-muted p-2 text-xs">{this.state.message}</pre>
          <button
            className="rounded bg-primary px-4 py-2 text-sm text-primary-foreground"
            onClick={this.reset}
          >
            重置
          </button>
        </div>
      )
    }
    return this.props.children
  }
}
