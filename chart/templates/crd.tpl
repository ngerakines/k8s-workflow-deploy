---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: "workflows.workflow-deploy.ngerakines.me"
  labels:
    {{- include "..labels" . | nindent 4 }}
spec:
  group: workflow-deploy.ngerakines.me
  names:
    categories: []
    kind: Workflow
    plural: workflows
    shortNames: []
    singular: workflow
  scope: Cluster
  versions:
  - additionalPrinterColumns: []
    name: v1alpha
    schema:
      openAPIV3Schema:
        description: Auto-generated derived type for WorkflowSpec via `CustomResource`
        properties:
          spec:
            properties:
              debounce:
                format: uint32
                minimum: 0
                nullable: true
                type: integer
              namespaces:
                items:
                  type: string
                type: array
              parallel:
                format: uint32
                minimum: 0
                nullable: true
                type: integer
              steps:
                items:
                  properties:
                    actions:
                      items:
                        properties:
                          action:
                            type: string
                          targets:
                            items:
                              properties:
                                containers:
                                  items:
                                    type: string
                                  type: array
                                name:
                                  type: string
                                resource:
                                  type: string
                              required:
                              - containers
                              - name
                              - resource
                              type: object
                            type: array
                        required:
                        - action
                        - targets
                        type: object
                      type: array
                  required:
                  - actions
                  type: object
                type: array
              supression:
                items:
                  type: string
                type: array
              version:
                type: string
            required:
            - namespaces
            - steps
            - supression
            - version
            type: object
        required:
        - spec
        title: Workflow
        type: object
    served: true
    storage: true
    subresources: {}