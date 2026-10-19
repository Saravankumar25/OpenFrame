import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button, EmptyState } from "../../design-system";

/** A rendering failure in one workspace must never take down the app or touch project data. */
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("workspace render failure", error.message, info.componentStack?.slice(0, 500));
  }
  render() {
    if (this.state.error) {
      return (
        <EmptyState
          title="This view couldn't be displayed."
          actions={<Button onClick={() => this.setState({ error: null })}>Try Again</Button>}
        >
          Your project was not changed and all your work is saved. Try again, or switch to another workspace.
        </EmptyState>
      );
    }
    return this.props.children;
  }
}
