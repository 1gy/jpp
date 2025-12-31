import { style, globalStyle } from '@vanilla-extract/css'
import { colors, fonts } from './theme'

globalStyle('*, *::before, *::after', {
  boxSizing: 'border-box',
  margin: 0,
  padding: 0,
})

globalStyle('html, body, #root', {
  height: '100%',
  width: '100%',
})

globalStyle('body', {
  backgroundColor: colors.bg,
  color: colors.text,
  fontFamily: fonts.mono,
  fontSize: '14px',
  lineHeight: 1.5,
})

export const container = style({
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
})

export const header = style({
  padding: '12px 20px',
  borderBottom: `1px solid ${colors.border}`,
  display: 'flex',
  alignItems: 'center',
  gap: '16px',
})

export const title = style({
  fontSize: '18px',
  fontWeight: 600,
  color: colors.text,
})

export const main = style({
  flex: 1,
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  minHeight: 0,
})

export const pane = style({
  display: 'flex',
  flexDirection: 'column',
  minHeight: 0,
  overflow: 'hidden',
})

export const leftPane = style([pane, {
  borderRight: `1px solid ${colors.border}`,
}])

export const rightPane = style([pane, {}])

export const paneHeader = style({
  padding: '8px 16px',
  borderBottom: `1px solid ${colors.border}`,
  backgroundColor: colors.surface,
  fontSize: '12px',
  color: colors.textMuted,
  textTransform: 'uppercase',
  letterSpacing: '0.5px',
})

export const paneContent = style({
  flex: 1,
  minHeight: 0,
  overflow: 'hidden',
})

export const queryInput = style({
  padding: '12px 16px',
  backgroundColor: colors.surface,
  border: 'none',
  borderBottom: `1px solid ${colors.border}`,
  color: colors.text,
  fontFamily: fonts.mono,
  fontSize: '14px',
  outline: 'none',
  ':focus': {
    borderBottomColor: colors.accent,
  },
  '::placeholder': {
    color: colors.textMuted,
  },
})

export const textarea = style({
  flex: 1,
  padding: '16px',
  backgroundColor: colors.bg,
  border: 'none',
  color: colors.text,
  fontFamily: fonts.mono,
  fontSize: '14px',
  resize: 'none',
  outline: 'none',
  '::placeholder': {
    color: colors.textMuted,
  },
})

export const resultArea = style({
  flex: 1,
  padding: '16px',
  backgroundColor: colors.bg,
  overflow: 'auto',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
})

export const errorText = style({
  color: colors.error,
})

export const loadingText = style({
  color: colors.textMuted,
  fontStyle: 'italic',
})
