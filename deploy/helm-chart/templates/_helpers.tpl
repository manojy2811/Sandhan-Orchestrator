{{- define "sandhan-orchestrator.fullname" -}}
{{- .Release.Name | trunc 63 | cleanSuffix "-" -}}
{{- end -}}
