<template>
    <div>
     <b-jumbotron>
      <div class="row">
            <div class="col-md-6">
                Name:
            </div>
            <div class="col-md-6">
                First Last
            </div>
        </div>
        <div class="row">
            <div class="col-md-6">
                Age:
            </div>
            <div class="col-md-6">
                99
            </div>
        </div>
        <div class="row">
            <div class="col-md-6">
                Occupation:
            </div>
            <div class="col-md-6">
                blah blah blah
            </div>
        </div>
    </b-jumbotron>
    <b-jumbotron>
        <div>
            <b-button v-b-toggle.collapse-1 class="m-1">Project 1</b-button>
            <b-collapse visible id="collapse-1">
                <b-card>blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blahblah blah blah blah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blah</b-card>
            </b-collapse>
        </div>
        <div>
            <b-button v-b-toggle.collapse-2 class="m-2">Project 2</b-button>
            <b-collapse visible id="collapse-2">
                <b-card>blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blahblah blah blah blah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blah</b-card>
            </b-collapse>
        </div>
        <div>
            <b-button v-b-toggle.collapse-3 class="m-3">Project 3</b-button>
            <b-collapse visible id="collapse-3">
                <b-card>blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blah blahblah blah blah blah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah blahblah
                    blah blahblah blahblah blahblah blahblah blahblah blahblah blah</b-card>
            </b-collapse>
        </div>
    </b-jumbotron>
    </div>
</template>

<script>
    import { BJumbotron, BButton, BCollapse, BCard, VBToggle } from 'bootstrap-vue'

    export default {
        name: 'Dashboard',
        components: {
          BJumbotron,
          BButton,
          BCollapse,
          BCard
        },
        directives: {
            "b-toggle": VBToggle
        }
    }
</script>