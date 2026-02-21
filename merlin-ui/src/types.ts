/** Severity level of a review comment, lowest-to-highest. */
export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';

/** Category of a review comment. */
export type Category = 'bug' | 'security' | 'style' | 'performance';

/** A single code review comment produced by the Merlin Rust binary. */
export interface ReviewComment {
  file: string;
  line: number;
  severity: Severity;
  category: Category;
  title: string;
  body: string;
  suggestion?: string | null;
}

/** A single message in the agent conversation history. */
export interface AgentMessage {
  role: 'user' | 'assistant';
  content: string;
}
